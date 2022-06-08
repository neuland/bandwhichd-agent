use std::collections::HashMap;
use std::process;
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::park_timeout;
use std::time::{Duration, Instant, SystemTime};

use pnet::datalink::{DataLinkReceiver, NetworkInterface};

use crate::agent_id::AgentId;
use crate::machine_id::MachineId;
use crate::network::{LocalSocket, Sniffer, Utilization};
use crate::publish::{
    Message, NetworkConfigurationV1MeasurementMessage, NetworkUtilizationV1MeasurementMessage,
};

mod agent_id;
mod machine_id;
mod network;
mod os;
mod publish;

const DEFAULT_NETWORK_CONFIGURATION_PUBLISH_INTERVAL: Duration = Duration::from_secs(600);
const DEFAULT_NETWORK_UTILIZATION_PUBLISH_INTERVAL: Duration = Duration::from_secs(10);
const DEFAULT_WATCHDOG_NOTIFY_INTERVAL: Duration = Duration::from_secs(10);
const WATCHDOG_MARGIN: Duration = Duration::from_secs(2);
const MAXIMUM_NUMBER_OF_CONSECUTIVE_PUBLISH_ERRORS: u8 = 3;

fn main() {
    if let Err(err) = try_main() {
        eprintln!("Error: {}", err);
        process::exit(95);
    }
}

fn try_main() -> Result<(), failure::Error> {
    let server = std::env::var("BANDWHICHD_SERVER")?;
    let os_input = os::get_input()?;
    start(server, os_input);
    Ok(())
}

pub struct OpenSockets {
    sockets_to_procs: HashMap<LocalSocket, String>,
}

pub struct OsInputOutput {
    pub network_interfaces: Vec<NetworkInterface>,
    pub network_frames: Vec<Box<dyn DataLinkReceiver>>,
    pub get_open_sockets: fn() -> OpenSockets,
}

fn abort() {
    if libsystemd::daemon::booted() {
        libsystemd::daemon::notify(
            false,
            &[
                libsystemd::daemon::NotifyState::Errno(131),
                libsystemd::daemon::NotifyState::Stopping,
            ],
        )
        .unwrap();
    }
    process::exit(131);
}

pub fn start(server: String, os_input: OsInputOutput) {
    let start = Instant::now();
    let systemd_enabled = libsystemd::daemon::booted();
    let agent_id = AgentId::default();
    let machine_id = MachineId::default();
    let publish_endpoint = format!("{}/v1/message", server);

    let mut active_threads = vec![];
    let last_publish_network_configuration = Arc::new(Mutex::new(start));
    let last_publish_network_utilization = Arc::new(Mutex::new(start));

    let get_open_sockets = os_input.get_open_sockets;

    let network_utilization = Arc::new(Mutex::new(Utilization::new()));

    active_threads.push(
        thread::Builder::new()
            .name("publish_network_configuration_handler".to_string())
            .spawn({
                let last_publish_network_configuration = last_publish_network_configuration.clone();
                let publish_interval = DEFAULT_NETWORK_CONFIGURATION_PUBLISH_INTERVAL;
                let publish_endpoint = publish_endpoint.clone();

                let client = reqwest::blocking::Client::new();
                let mut error_count = 0;

                move || loop {
                    let publish_start_time = Instant::now();
                    let open_sockets = get_open_sockets();

                    *last_publish_network_configuration.lock().unwrap() = publish_start_time;

                    {
                        let message = Message::NetworkConfigurationV1Measurement(
                            NetworkConfigurationV1MeasurementMessage::from(
                                agent_id,
                                SystemTime::now(),
                                machine_id.clone(),
                                gethostname::gethostname().into_string().unwrap(),
                                pnet::datalink::interfaces(),
                                open_sockets,
                            ),
                        );
                        let publish_result =
                            client.post(publish_endpoint.clone()).json(&message).send();
                        match publish_result {
                            Ok(response) if response.status() == 200 => {
                                error_count = 0;
                            }
                            Ok(response) => {
                                error_count += 1;
                                eprintln!("Publish error, response: {:?}", response);
                                if systemd_enabled {
                                    libsystemd::daemon::notify(
                                        false,
                                        &[libsystemd::daemon::NotifyState::Errno(5)],
                                    )
                                    .unwrap();
                                }
                            }
                            Err(error) => {
                                error_count += 1;
                                eprintln!("Publish error, error: {:?}", error);
                                if systemd_enabled {
                                    libsystemd::daemon::notify(
                                        false,
                                        &[libsystemd::daemon::NotifyState::Errno(5)],
                                    )
                                    .unwrap();
                                }
                            }
                        }

                        if error_count > MAXIMUM_NUMBER_OF_CONSECUTIVE_PUBLISH_ERRORS {
                            abort();
                        }
                    }

                    let publish_duration = publish_start_time.elapsed();
                    if publish_duration < publish_interval {
                        park_timeout(publish_interval - publish_duration);
                    }
                }
            })
            .unwrap(),
    );

    active_threads.push(
        thread::Builder::new()
            .name("publish_network_utilization_handler".to_string())
            .spawn({
                let last_publish_network_utilization = last_publish_network_utilization.clone();
                let network_utilization = network_utilization.clone();
                let publish_interval = DEFAULT_NETWORK_UTILIZATION_PUBLISH_INTERVAL;

                let client = reqwest::blocking::Client::new();
                let mut error_count = 0;

                move || {
                    park_timeout(publish_interval);
                    loop {
                        let publish_start_time = Instant::now();
                        let utilization = { network_utilization.lock().unwrap().clone_and_reset() };

                        *last_publish_network_utilization.lock().unwrap() = publish_start_time;

                        {
                            let message = Message::NetworkUtilizationV1Measurement(
                                NetworkUtilizationV1MeasurementMessage::from(agent_id, utilization),
                            );
                            let publish_result =
                                client.post(publish_endpoint.clone()).json(&message).send();
                            match publish_result {
                                Ok(response) if response.status() == 200 => {
                                    error_count = 0;
                                }
                                Ok(response) => {
                                    error_count += 1;
                                    eprintln!("Publish error, response: {:?}", response);
                                    if systemd_enabled {
                                        libsystemd::daemon::notify(
                                            false,
                                            &[libsystemd::daemon::NotifyState::Errno(5)],
                                        )
                                        .unwrap();
                                    }
                                }
                                Err(error) => {
                                    error_count += 1;
                                    eprintln!("Publish error, error: {:?}", error);
                                    if systemd_enabled {
                                        libsystemd::daemon::notify(
                                            false,
                                            &[libsystemd::daemon::NotifyState::Errno(5)],
                                        )
                                        .unwrap();
                                    }
                                }
                            }

                            if error_count > MAXIMUM_NUMBER_OF_CONSECUTIVE_PUBLISH_ERRORS {
                                abort();
                            }
                        }

                        let publish_duration = publish_start_time.elapsed();
                        if publish_duration < publish_interval {
                            park_timeout(publish_interval - publish_duration);
                        }
                    }
                }
            })
            .unwrap(),
    );

    let sniffer_threads = os_input
        .network_interfaces
        .into_iter()
        .zip(os_input.network_frames.into_iter())
        .map(|(interface, frames)| {
            let name = format!("sniffing_handler_{}", interface.name);
            let network_utilization = network_utilization.clone();

            thread::Builder::new()
                .name(name)
                .spawn(move || {
                    let mut sniffer = Sniffer::new(interface, frames);

                    loop {
                        if let Some(segment) = sniffer.next() {
                            network_utilization.lock().unwrap().update(segment);
                        }
                    }
                })
                .unwrap()
        })
        .collect::<Vec<_>>();
    active_threads.extend(sniffer_threads);

    active_threads.push(
        thread::Builder::new()
            .name("watchdog".to_string())
            .spawn({
                let notify_interval = DEFAULT_WATCHDOG_NOTIFY_INTERVAL;

                move || loop {
                    let notify_start_time = Instant::now();

                    let publish_network_configuration_elapsed =
                        last_publish_network_configuration.lock().unwrap().elapsed();
                    let publish_network_utilization_elapsed =
                        last_publish_network_utilization.lock().unwrap().elapsed();

                    let publish_network_configuration_unresponsive =
                        publish_network_configuration_elapsed
                            > DEFAULT_NETWORK_CONFIGURATION_PUBLISH_INTERVAL + WATCHDOG_MARGIN;
                    let publish_network_utilization_unresponsive =
                        publish_network_utilization_elapsed
                            > DEFAULT_NETWORK_UTILIZATION_PUBLISH_INTERVAL + WATCHDOG_MARGIN;

                    if publish_network_configuration_unresponsive
                        || publish_network_utilization_unresponsive
                    {
                        if publish_network_configuration_unresponsive {
                            eprintln!("Publish network configuration unresponsive");
                        }
                        if publish_network_utilization_unresponsive {
                            eprintln!("Publish network utilization unresponsive");
                        }
                        if systemd_enabled {
                            libsystemd::daemon::notify(
                                false,
                                &[
                                    libsystemd::daemon::NotifyState::Errno(131),
                                    libsystemd::daemon::NotifyState::Stopping,
                                ],
                            )
                            .unwrap();
                        }
                        process::exit(131);
                    } else {
                        if systemd_enabled {
                            libsystemd::daemon::notify(
                                false,
                                &[libsystemd::daemon::NotifyState::Watchdog],
                            )
                            .unwrap();
                        }

                        let notify_duration = notify_start_time.elapsed();
                        if notify_duration < notify_interval {
                            park_timeout(notify_interval - notify_duration);
                        }
                    }
                }
            })
            .unwrap(),
    );

    if systemd_enabled {
        libsystemd::daemon::notify(false, &[libsystemd::daemon::NotifyState::Ready]).unwrap();
    }

    for thread_handler in active_threads {
        thread_handler.join().unwrap();
    }
}
