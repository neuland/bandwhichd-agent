#![deny(clippy::all)]

use std::collections::HashMap;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::park_timeout;
use std::time::{Duration, Instant, SystemTime};

use clap::Parser;
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

#[derive(Parser)]
#[clap(version, about, long_about = None)]
#[clap(propagate_version = true)]
pub struct Opt {
    #[clap(long)]
    /// Publish endpoint
    publish_endpoint: String,
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("Error: {}", err);
        process::exit(2);
    }
}

fn try_main() -> Result<(), failure::Error> {
    let opts = Opt::parse();
    let os_input = os::get_input()?;
    start(os_input, opts);
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

pub fn start(os_input: OsInputOutput, opts: Opt) {
    let agent_id = AgentId::default();
    let machine_id = MachineId::default();
    let running = Arc::new(AtomicBool::new(true));

    let mut active_threads = vec![];

    let get_open_sockets = os_input.get_open_sockets;

    let network_utilization = Arc::new(Mutex::new(Utilization::new()));

    active_threads.push(
        thread::Builder::new()
            .name("publish_network_configuration_handler".to_string())
            .spawn({
                let running = running.clone();
                let publish_interval = DEFAULT_NETWORK_CONFIGURATION_PUBLISH_INTERVAL;
                let publish_endpoint = opts.publish_endpoint.clone();

                let client = reqwest::blocking::Client::new();

                move || {
                    while running.load(Ordering::Acquire) {
                        let publish_start_time = Instant::now();
                        let open_sockets = get_open_sockets();

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
                                Ok(response) if response.status() == 200 => {}
                                Ok(response) => println!("Publish error, response: {:?}", response),
                                Err(error) => eprintln!("Publish error, error: {:?}", error),
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

    active_threads.push(
        thread::Builder::new()
            .name("publish_network_utilization_handler".to_string())
            .spawn({
                let running = running.clone();
                let network_utilization = network_utilization.clone();
                let publish_interval = DEFAULT_NETWORK_UTILIZATION_PUBLISH_INTERVAL;
                let publish_endpoint = opts.publish_endpoint;

                let client = reqwest::blocking::Client::new();

                move || {
                    park_timeout(publish_interval);
                    while running.load(Ordering::Acquire) {
                        let publish_start_time = Instant::now();
                        let utilization = { network_utilization.lock().unwrap().clone_and_reset() };

                        {
                            let message = Message::NetworkUtilizationV1Measurement(
                                NetworkUtilizationV1MeasurementMessage::from(agent_id, utilization),
                            );
                            let publish_result =
                                client.post(publish_endpoint.clone()).json(&message).send();
                            match publish_result {
                                Ok(response) if response.status() == 200 => {}
                                Ok(response) => println!("Publish error, response: {:?}", response),
                                Err(error) => eprintln!("Publish error, error: {:?}", error),
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
            let running = running.clone();
            let network_utilization = network_utilization.clone();

            thread::Builder::new()
                .name(name)
                .spawn(move || {
                    let mut sniffer = Sniffer::new(interface, frames);

                    while running.load(Ordering::Acquire) {
                        if let Some(segment) = sniffer.next() {
                            network_utilization.lock().unwrap().update(segment);
                        }
                    }
                })
                .unwrap()
        })
        .collect::<Vec<_>>();
    active_threads.extend(sniffer_threads);

    for thread_handler in active_threads {
        thread_handler.join().unwrap()
    }
}
