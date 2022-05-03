#![deny(clippy::all)]

mod network;
mod os;
mod publish;

use crate::publish::VersionedPublishMessage;
use ::pnet::datalink::{DataLinkReceiver, NetworkInterface};
use ::std::collections::HashMap;
use ::std::sync::atomic::{AtomicBool, Ordering};
use ::std::sync::{Arc, Mutex};
use ::std::thread;
use ::std::thread::park_timeout;
use ::std::time::{Duration, Instant};
use network::{LocalSocket, Sniffer, Utilization};
use std::process;
use structopt::StructOpt;

const DEFAULT_PUBLISH_INTERVAL: Duration = Duration::from_secs(10);

#[derive(StructOpt, Debug)]
#[structopt(name = "bandwhichd-agent")]
pub struct Opt {
    #[structopt(long)]
    /// Publish endpoint
    publish_endpoint: String,
    #[structopt(long)]
    /// The network interface to listen on, eg. eth0
    interface: Option<String>,
    #[structopt(long)]
    /// Publish interval in seconds, default is 10
    interval: Option<usize>,
}

fn main() {
    if let Err(err) = try_main() {
        eprintln!("Error: {}", err);
        process::exit(2);
    }
}

fn try_main() -> Result<(), failure::Error> {
    use os::get_input;
    let opts = Opt::from_args();
    let os_input = get_input(&opts.interface)?;
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
    let running = Arc::new(AtomicBool::new(true));

    let mut active_threads = vec![];

    let get_open_sockets = os_input.get_open_sockets;

    let network_utilization = Arc::new(Mutex::new(Utilization::new()));

    active_threads.push(
        thread::Builder::new()
            .name("publish_handler".to_string())
            .spawn({
                let running = running.clone();
                let network_utilization = network_utilization.clone();
                let publish_endpoint = opts.publish_endpoint.clone();
                let publish_interval = opts.interval.map_or_else(
                    || DEFAULT_PUBLISH_INTERVAL,
                    |s| Duration::from_secs(s as u64),
                );

                let client = reqwest::blocking::Client::new();

                move || {
                    while running.load(Ordering::Acquire) {
                        let publish_start_time = Instant::now();
                        let utilization = { network_utilization.lock().unwrap().clone_and_reset() };
                        let open_sockets = get_open_sockets();

                        let package = VersionedPublishMessage::from(utilization, open_sockets);
                        let publish_result =
                            client.post(publish_endpoint.clone()).json(&package).send();
                        match publish_result {
                            Ok(response) if response.status() == 200 => {}
                            Ok(response) => println!("Publish error, response: {:?}", response),
                            Err(error) => eprintln!("Publish error, error: {:?}", error),
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
