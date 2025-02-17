use std::fmt::{Display, Formatter, Write};
use std::net::SocketAddr;
use std::time::SystemTime;

use ipnetwork::IpNetwork;
use pnet::datalink::NetworkInterface;
use serde::{Serialize, Serializer};
use time::{Duration, OffsetDateTime};

use crate::network::Protocol;
use crate::{MachineId, OpenSockets, OsRelease, Utilization};

#[derive(Serialize)]
#[serde(tag = "type", content = "content")]
pub enum Message {
    #[serde(rename = "bandwhichd/measurement/agent-network-configuration/v1")]
    NetworkConfigurationV1Measurement(NetworkConfigurationV1MeasurementMessage),
    #[serde(rename = "bandwhichd/measurement/agent-network-utilization/v1")]
    NetworkUtilizationV1Measurement(NetworkUtilizationV1MeasurementMessage),
}

#[derive(Serialize)]
pub struct NetworkConfigurationV1MeasurementMessage {
    pub machine_id: MachineId,
    pub timestamp: TimestampV1,
    pub maybe_os_release: Option<OsRelease>,
    pub hostname: String,
    pub interfaces: Vec<InterfaceV1>,
    pub open_sockets: Vec<OpenSocketV1>,
}

impl NetworkConfigurationV1MeasurementMessage {
    pub fn from(
        machine_id: MachineId,
        timestamp: SystemTime,
        maybe_os_release: Option<OsRelease>,
        hostname: String,
        network_interfaces: Vec<NetworkInterface>,
        open_sockets: OpenSockets,
    ) -> Self {
        let mut interfaces: Vec<InterfaceV1> = network_interfaces
            .into_iter()
            .map(|network_interface| InterfaceV1 {
                name: network_interface.name.clone(),
                is_up: network_interface.is_up(),
                networks: network_interface.ips,
            })
            .collect();
        interfaces.sort();
        let mut open_sockets: Vec<OpenSocketV1> = open_sockets
            .sockets_to_procs
            .into_iter()
            .map(|(socket, process)| OpenSocketV1 {
                socket_address: socket.into(),
                protocol: ProtocolV1(socket.protocol),
                process,
            })
            .collect();
        open_sockets.sort();
        NetworkConfigurationV1MeasurementMessage {
            hostname,
            machine_id,
            maybe_os_release,
            timestamp: TimestampV1(timestamp.into()),
            interfaces,
            open_sockets,
        }
    }
}

#[derive(Serialize)]
pub struct NetworkUtilizationV1MeasurementMessage {
    pub machine_id: MachineId,
    pub timeframe: TimeframeV1,
    pub connections: Vec<ConnectionV1>,
}

impl NetworkUtilizationV1MeasurementMessage {
    pub fn from(machine_id: MachineId, utilization: Utilization) -> Self {
        let mut connections: Vec<ConnectionV1> = utilization
            .connections
            .into_iter()
            .map(|(connection, connection_info)| ConnectionV1 {
                interface_name: connection_info.interface_name,
                local_socket_address: connection.local_socket.into(),
                remote_socket_address: connection.remote_socket.into(),
                protocol: ProtocolV1(connection.local_socket.protocol),
                received: BytesCount(connection_info.total_bytes_downloaded),
                sent: BytesCount(connection_info.total_bytes_uploaded),
            })
            .collect();
        connections.sort();
        let start: OffsetDateTime = utilization.start.into();
        let stop: OffsetDateTime = utilization.stop.into();
        NetworkUtilizationV1MeasurementMessage {
            machine_id,
            timeframe: TimeframeV1 {
                start: TimestampV1(start),
                duration: DurationV1(stop - start),
            },
            connections,
        }
    }
}

pub struct TimestampV1(OffsetDateTime);

impl Display for TimestampV1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Display::fmt(
            self.0
                .format(&time::format_description::well_known::Rfc3339)
                .map_err(|_| std::fmt::Error)?
                .as_str(),
            f,
        )
    }
}

impl Serialize for TimestampV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        time::serde::rfc3339::serialize(&self.0, serializer)
    }
}

pub struct TimeframeV1 {
    pub start: TimestampV1,
    pub duration: DurationV1,
}

impl Display for TimeframeV1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        self.start.fmt(f)?;
        f.write_char('/')?;
        self.duration.fmt(f)
    }
}

impl Serialize for TimeframeV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

pub struct DurationV1(Duration);

impl Display for DurationV1 {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_char('P')?;
        f.write_char('T')?;
        Display::fmt(&self.0.as_seconds_f32(), f)?;
        f.write_char('S')
    }
}

impl Serialize for MachineId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.secure_uuid().serialize(serializer)
    }
}

impl Serialize for OsRelease {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.file_contents().serialize(serializer)
    }
}

#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct InterfaceV1 {
    pub name: String,
    pub is_up: bool,
    pub networks: Vec<IpNetwork>,
}

#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct ConnectionV1 {
    pub interface_name: String,
    pub local_socket_address: SocketAddr,
    pub remote_socket_address: SocketAddr,
    pub protocol: ProtocolV1,
    pub received: BytesCount,
    pub sent: BytesCount,
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct ProtocolV1(Protocol);

impl Serialize for ProtocolV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(match self.0 {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        })
    }
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct BytesCount(u128);

impl Serialize for BytesCount {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.to_string().as_str())
    }
}

#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct OpenSocketV1 {
    pub socket_address: SocketAddr,
    pub protocol: ProtocolV1,
    pub process: String,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
    use std::str::FromStr;
    use std::time::SystemTime;

    use assert_json_diff::assert_json_eq;
    use ipnetwork::{IpNetwork, Ipv4Network, Ipv6Network};
    use serde_json::json;
    use serde_json::{from_str, Value};
    use time::macros::datetime;

    use crate::network::{Connection, ConnectionInfo, Socket};
    use crate::LocalSocket;

    use super::*;

    #[test]
    fn should_serialize_network_configuration_v1_measurement_message_json() {
        // given
        let message = Message::NetworkConfigurationV1Measurement(
            NetworkConfigurationV1MeasurementMessage::from(
                MachineId::new("<machine-id>".to_string()),
                SystemTime::from(datetime!(2022-05-06 15:14:51.74223728 utc)),
                Some(OsRelease::new("<os-release>".to_string())),
                "some-host.example.com".to_string(),
                vec![
                    NetworkInterface {
                        name: "lo".to_string(),
                        description: "".to_string(),
                        index: 0,
                        mac: None,
                        ips: vec![
                            IpNetwork::V4(Ipv4Network::new(Ipv4Addr::LOCALHOST, 8).unwrap()),
                            IpNetwork::V6(Ipv6Network::new(Ipv6Addr::LOCALHOST, 128).unwrap()),
                        ],
                        flags: pnet_sys::IFF_UP as u32,
                    },
                    NetworkInterface {
                        name: "enp0s31f6".to_string(),
                        description: "".to_string(),
                        index: 0,
                        mac: None,
                        ips: vec![],
                        flags: 0,
                    },
                    NetworkInterface {
                        name: "wlp3s0".to_string(),
                        description: "".to_string(),
                        index: 0,
                        mac: None,
                        ips: vec![
                            IpNetwork::V4(
                                Ipv4Network::new(Ipv4Addr::new(172, 18, 195, 209), 16).unwrap(),
                            ),
                            IpNetwork::V6(
                                Ipv6Network::new(
                                    Ipv6Addr::from_str("fe80::8e71:453d:204d:abf8").unwrap(),
                                    64,
                                )
                                .unwrap(),
                            ),
                        ],
                        flags: pnet_sys::IFF_UP as u32,
                    },
                    NetworkInterface {
                        name: "virbr0".to_string(),
                        description: "".to_string(),
                        index: 0,
                        mac: None,
                        ips: vec![IpNetwork::V4(
                            Ipv4Network::new(Ipv4Addr::new(192, 168, 122, 1), 24).unwrap(),
                        )],
                        flags: 0,
                    },
                    NetworkInterface {
                        name: "docker0".to_string(),
                        description: "".to_string(),
                        index: 0,
                        mac: None,
                        ips: vec![
                            IpNetwork::V4(
                                Ipv4Network::new(Ipv4Addr::new(172, 17, 0, 1), 16).unwrap(),
                            ),
                            IpNetwork::V6(
                                Ipv6Network::new(
                                    Ipv6Addr::from_str("fe80::42:a4ff:fef2:4ad4").unwrap(),
                                    64,
                                )
                                .unwrap(),
                            ),
                        ],
                        flags: 0,
                    },
                ],
                OpenSockets {
                    sockets_to_procs: HashMap::from([
                        (
                            LocalSocket {
                                ip: IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 0)),
                                port: 37863,
                                protocol: Protocol::Tcp,
                            },
                            "java".to_string(),
                        ),
                        (
                            LocalSocket {
                                ip: IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)),
                                port: 68,
                                protocol: Protocol::Udp,
                            },
                            "dhclient".to_string(),
                        ),
                    ]),
                },
            ),
        );

        // when
        let result = serde_json::to_string(&message);

        // then
        let actual: Value = from_str(result.unwrap().as_str()).unwrap();
        let expected: Value = json!({
            "type": "bandwhichd/measurement/agent-network-configuration/v1",
            "content": {
                "machine_id": "d2c1d575-326e-b00b-c3eb-26ef934301f0",
                "timestamp": "2022-05-06T15:14:51.74223728Z",
                "maybe_os_release": "<os-release>",
                "hostname": "some-host.example.com",
                "interfaces": [
                    {
                        "name": "docker0",
                        "is_up": false,
                        "networks": [
                            "172.17.0.1/16",
                            "fe80::42:a4ff:fef2:4ad4/64",
                        ]
                    },
                    {
                        "name": "enp0s31f6",
                        "is_up": false,
                        "networks": [],
                    },
                    {
                        "name": "lo",
                        "is_up": true,
                        "networks": [
                            "127.0.0.1/8",
                            "::1/128",
                        ],
                    },
                    {
                        "name": "virbr0",
                        "is_up": false,
                        "networks": [
                            "192.168.122.1/24",
                        ],
                    },
                    {
                        "name": "wlp3s0",
                        "is_up": true,
                        "networks": [
                            "172.18.195.209/16",
                            "fe80::8e71:453d:204d:abf8/64",
                        ],
                    },
                ],
                "open_sockets": [
                    {
                        "socket_address": "0.0.0.0:68",
                        "protocol": "udp",
                        "process": "dhclient"
                    },
                    {
                        "socket_address": "[::]:37863",
                        "protocol": "tcp",
                        "process": "java"
                    }
                ]
            }
        });
        assert_json_eq!(actual, expected);
    }

    #[test]
    fn should_serialize_network_utilization_v1_measurement_message_json() {
        // given
        let message =
            Message::NetworkUtilizationV1Measurement(NetworkUtilizationV1MeasurementMessage::from(
                MachineId::new("<machine-id>".to_string()),
                Utilization {
                    connections: HashMap::from([
                        (
                            Connection {
                                remote_socket: Socket {
                                    ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                                    port: 8080,
                                },
                                local_socket: LocalSocket {
                                    ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                                    port: 36070,
                                    protocol: Protocol::Tcp,
                                },
                            },
                            ConnectionInfo {
                                interface_name: "lo".to_string(),
                                total_bytes_downloaded: 0,
                                total_bytes_uploaded: 13882,
                            },
                        ),
                        (
                            Connection {
                                remote_socket: Socket {
                                    ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                                    port: 36070,
                                },
                                local_socket: LocalSocket {
                                    ip: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
                                    port: 8080,
                                    protocol: Protocol::Tcp,
                                },
                            },
                            ConnectionInfo {
                                interface_name: "lo".to_string(),
                                total_bytes_downloaded: 608,
                                total_bytes_uploaded: 0,
                            },
                        ),
                        (
                            Connection {
                                remote_socket: Socket {
                                    ip: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 34)),
                                    port: 5353,
                                },
                                local_socket: LocalSocket {
                                    ip: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 87)),
                                    port: 43254,
                                    protocol: Protocol::Udp,
                                },
                            },
                            ConnectionInfo {
                                interface_name: "tun0".to_string(),
                                total_bytes_downloaded: 120,
                                total_bytes_uploaded: 64,
                            },
                        ),
                    ]),
                    start: SystemTime::from(datetime!(2022-05-06 15:14:51.74223728 utc)),
                    stop: SystemTime::from(datetime!(2022-05-06 15:15:01.84260156 utc)),
                },
            ));

        // when
        let result = serde_json::to_string(&message);

        // then
        let actual: Value = from_str(result.unwrap().as_str()).unwrap();
        let expected: Value = json!({
            "type": "bandwhichd/measurement/agent-network-utilization/v1",
            "content": {
                "machine_id": "d2c1d575-326e-b00b-c3eb-26ef934301f0",
                "timeframe": "2022-05-06T15:14:51.74223728Z/PT10.100365S",
                "connections": [
                    {
                        "interface_name": "lo",
                        "local_socket_address": "127.0.0.1:8080",
                        "remote_socket_address": "127.0.0.1:36070",
                        "protocol": "tcp",
                        "received": "608",
                        "sent": "0"
                    },
                    {
                        "interface_name": "lo",
                        "local_socket_address": "127.0.0.1:36070",
                        "remote_socket_address": "127.0.0.1:8080",
                        "protocol": "tcp",
                        "received": "0",
                        "sent": "13882"
                    },
                    {
                        "interface_name": "tun0",
                        "local_socket_address": "192.168.10.87:43254",
                        "remote_socket_address": "192.168.10.34:5353",
                        "protocol": "udp",
                        "received": "120",
                        "sent": "64"
                    }
                ],
            }
        });
        assert_json_eq!(actual, expected);
    }
}
