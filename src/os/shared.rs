use std::io::ErrorKind;
use std::time;

use pnet::datalink::Channel::Ethernet;
use pnet::datalink::DataLinkReceiver;
use pnet::datalink::{self, Config, NetworkInterface};

use crate::os::errors::GetInterfaceErrorKind;
use crate::os::linux::get_open_sockets;
use crate::OsInputOutput;

pub(crate) fn get_datalink_channel(
    interface: &NetworkInterface,
) -> Result<Box<dyn DataLinkReceiver>, GetInterfaceErrorKind> {
    let config = Config {
        read_timeout: Some(time::Duration::new(1, 0)),
        read_buffer_size: 65536,
        ..Default::default()
    };

    match datalink::channel(interface, config) {
        Ok(Ethernet(_tx, rx)) => Ok(rx),
        Ok(_) => Err(GetInterfaceErrorKind::OtherError(format!(
            "{}: Unsupported interface type",
            interface.name
        ))),
        Err(e) => match e.kind() {
            ErrorKind::PermissionDenied => Err(GetInterfaceErrorKind::PermissionError(
                interface.name.to_owned(),
            )),
            _ => Err(GetInterfaceErrorKind::OtherError(format!(
                "{}: {}",
                &interface.name, e
            ))),
        },
    }
}

#[derive(Debug)]
pub struct UserErrors {
    permission: Option<String>,
    other: Option<String>,
}

pub fn collect_errors<'a, I>(network_frames: I) -> String
where
    I: Iterator<
        Item = (
            &'a NetworkInterface,
            Result<Box<dyn DataLinkReceiver>, GetInterfaceErrorKind>,
        ),
    >,
{
    let errors = network_frames.fold(
        UserErrors {
            permission: None,
            other: None,
        },
        |acc, (_, elem)| {
            if let Some(iface_error) = elem.err() {
                return match iface_error {
                    GetInterfaceErrorKind::PermissionError(interface_name) => {
                        if let Some(prev_interface) = acc.permission {
                            UserErrors {
                                permission: Some(format!("{}, {}", prev_interface, interface_name)),
                                ..acc
                            }
                        } else {
                            UserErrors {
                                permission: Some(interface_name),
                                ..acc
                            }
                        }
                    }
                    error => {
                        if let Some(prev_errors) = acc.other {
                            UserErrors {
                                other: Some(format!("{} \n {:?}", prev_errors, error)),
                                ..acc
                            }
                        } else {
                            UserErrors {
                                other: Some(format!("{:?}", error)),
                                ..acc
                            }
                        }
                    }
                };
            }
            acc
        },
    );
    if let Some(interface_name) = errors.permission {
        if let Some(other_errors) = errors.other {
            format!(
                "\n\n{}: {} \nAdditional Errors: \n {}",
                interface_name,
                eperm_message(),
                other_errors
            )
        } else {
            format!("\n\n{}: {}", interface_name, eperm_message())
        }
    } else {
        let other_errors = errors
            .other
            .expect("asked to collect errors but found no errors");
        format!("\n\n {}", other_errors)
    }
}

pub fn get_input() -> Result<OsInputOutput, failure::Error> {
    let network_interfaces = datalink::interfaces();

    let network_frames = network_interfaces
        .iter()
        .filter(|iface| iface.is_up() && !iface.ips.is_empty())
        .map(|iface| (iface, get_datalink_channel(iface)));

    let (available_network_frames, network_interfaces) = {
        let network_frames = network_frames.clone();
        let mut available_network_frames = Vec::new();
        let mut available_interfaces: Vec<NetworkInterface> = Vec::new();
        for (iface, rx) in network_frames.filter_map(|(iface, channel)| {
            if let Ok(rx) = channel {
                Some((iface, rx))
            } else {
                None
            }
        }) {
            available_interfaces.push(iface.clone());
            available_network_frames.push(rx);
        }
        (available_network_frames, available_interfaces)
    };

    if available_network_frames.is_empty() {
        let all_errors = collect_errors(network_frames.clone());
        if !all_errors.is_empty() {
            failure::bail!(all_errors);
        }

        failure::bail!("Failed to find any network interface to listen on.");
    }

    Ok(OsInputOutput {
        network_interfaces,
        network_frames: available_network_frames,
        get_open_sockets,
    })
}

#[inline]
fn eperm_message() -> &'static str {
    r#"
    Insufficient permissions to listen on network interface(s). You can work around
    this issue like this:

    * Try running `bandwhichd-agent` with `sudo`

    * Build a `setcap(8)` wrapper for `bandwhichd-agent` with the following rules:
        `cap_sys_ptrace,cap_dac_read_search,cap_net_raw,cap_net_admin+ep`
    "#
}
