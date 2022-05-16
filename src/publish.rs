use pnet::datalink::NetworkInterface;
use std::net::IpAddr;

use serde::{Serialize, Serializer};
use time::OffsetDateTime;

use crate::network::Protocol;
use crate::{OpenSockets, Utilization};

#[derive(Serialize)]
#[serde(tag = "version", content = "message")]
pub enum VersionedMessage {
    #[serde(rename = "1")]
    V1(MessageV1),
}

impl VersionedMessage {
    pub fn from(
        agent_name: String,
        network_interfaces: Vec<NetworkInterface>,
        utilization: Utilization,
        open_sockets: OpenSockets,
    ) -> Self {
        VersionedMessage::V1(MessageV1::from(
            agent_name,
            network_interfaces,
            utilization,
            open_sockets,
        ))
    }
}

#[derive(Serialize)]
pub struct MessageV1 {
    pub agent_name: String,
    pub start: TimestampV1,
    pub stop: TimestampV1,
    pub interfaces: Vec<InterfaceV1>,
    pub connections: Vec<ConnectionV1>,
    pub open_sockets: Vec<OpenSocketV1>,
}

impl MessageV1 {
    pub fn from(
        agent_name: String,
        network_interfaces: Vec<NetworkInterface>,
        utilization: Utilization,
        open_sockets: OpenSockets,
    ) -> Self {
        let mut interfaces: Vec<InterfaceV1> = network_interfaces
            .into_iter()
            .map(|network_interface| InterfaceV1 {
                name: network_interface.name.clone(),
                is_up: network_interface.is_up(),
                ip_address_ranges: network_interface
                    .ips
                    .into_iter()
                    .map(|ip_network| IpAddressRangeV1 {
                        ip_address: IpAddressV1(ip_network.ip()),
                        prefix: ip_network.prefix(),
                    })
                    .collect(),
            })
            .collect();
        interfaces.sort();
        let mut connections: Vec<ConnectionV1> = utilization
            .connections
            .into_iter()
            .map(|(connection, connection_info)| ConnectionV1 {
                interface_name: connection_info.interface_name,
                local_ip_address: IpAddressV1(connection.local_socket.ip),
                local_port: connection.local_socket.port,
                remote_ip_address: IpAddressV1(connection.remote_socket.ip),
                remote_port: connection.remote_socket.port,
                protocol: ProtocolV1(connection.local_socket.protocol),
                received: BytesCount(connection_info.total_bytes_downloaded),
                sent: BytesCount(connection_info.total_bytes_uploaded),
            })
            .collect();
        connections.sort();
        let mut open_sockets: Vec<OpenSocketV1> = open_sockets
            .sockets_to_procs
            .into_iter()
            .map(|(socket, process)| OpenSocketV1 {
                ip_address: IpAddressV1(socket.ip),
                port: socket.port,
                protocol: ProtocolV1(socket.protocol),
                process,
            })
            .collect();
        open_sockets.sort();
        MessageV1 {
            agent_name,
            start: TimestampV1(utilization.start.into()),
            stop: TimestampV1(utilization.stop.into()),
            interfaces,
            connections,
            open_sockets,
        }
    }
}

pub struct TimestampV1(OffsetDateTime);

impl Serialize for TimestampV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        time::serde::rfc3339::serialize(&self.0, serializer)
    }
}

#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct InterfaceV1 {
    pub name: String,
    pub is_up: bool,
    pub ip_address_ranges: Vec<IpAddressRangeV1>,
}

#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct IpAddressRangeV1 {
    pub ip_address: IpAddressV1,
    pub prefix: u8,
}

#[derive(Serialize, Ord, PartialOrd, Eq, PartialEq)]
pub struct ConnectionV1 {
    pub interface_name: String,
    pub local_ip_address: IpAddressV1,
    pub local_port: u16,
    pub remote_ip_address: IpAddressV1,
    pub remote_port: u16,
    pub protocol: ProtocolV1,
    pub received: BytesCount,
    pub sent: BytesCount,
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct IpAddressV1(IpAddr);

impl Serialize for IpAddressV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.0.to_string().as_str())
    }
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
    pub ip_address: IpAddressV1,
    pub port: u16,
    pub protocol: ProtocolV1,
    pub process: String,
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::{Ipv4Addr, Ipv6Addr};
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
    fn should_serialize_v1_json() {
        // given
        let message = VersionedMessage::from(
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
                        IpNetwork::V4(Ipv4Network::new(Ipv4Addr::new(172, 17, 0, 1), 16).unwrap()),
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
                stop: SystemTime::from(datetime!(2022-05-06 15:15:01.74260156 utc)),
            },
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
        );

        // when
        let result = serde_json::to_string(&message);

        // then
        let actual: Value = from_str(result.unwrap().as_str()).unwrap();
        let expected: Value = json!({
            "version": "1",
            "message": {
                "agent_name": "some-host.example.com",
                "start": "2022-05-06T15:14:51.74223728Z",
                "stop": "2022-05-06T15:15:01.74260156Z",
                "interfaces": [
                    {
                        "name": "docker0",
                        "is_up": false,
                        "ip_address_ranges": [
                            {
                                "ip_address": "172.17.0.1",
                                "prefix": 16,
                            },
                            {
                                "ip_address": "fe80::42:a4ff:fef2:4ad4",
                                "prefix": 64,
                            }
                        ]
                    },
                    {
                        "name": "enp0s31f6",
                        "is_up": false,
                        "ip_address_ranges": [],
                    },
                    {
                        "name": "lo",
                        "is_up": true,
                        "ip_address_ranges": [
                            {
                                "ip_address": "127.0.0.1",
                                "prefix": 8
                            },
                            {
                                "ip_address": "::1",
                                "prefix": 128
                            },
                        ],
                    },
                    {
                        "name": "virbr0",
                        "is_up": false,
                        "ip_address_ranges": [
                            {
                                "ip_address": "192.168.122.1",
                                "prefix": 24
                            },
                        ],
                    },
                    {
                        "name": "wlp3s0",
                        "is_up": true,
                        "ip_address_ranges": [
                            {
                                "ip_address": "172.18.195.209",
                                "prefix": 16
                            },
                            {
                                "ip_address": "fe80::8e71:453d:204d:abf8",
                                "prefix": 64
                            },
                        ],
                    },
                ],
                "connections": [
                    {
                        "interface_name": "lo",
                        "local_ip_address": "127.0.0.1",
                        "local_port": 8080,
                        "remote_ip_address": "127.0.0.1",
                        "remote_port": 36070,
                        "protocol": "tcp",
                        "received": "608",
                        "sent": "0"
                    },
                    {
                        "interface_name": "lo",
                        "local_ip_address": "127.0.0.1",
                        "local_port": 36070,
                        "remote_ip_address": "127.0.0.1",
                        "remote_port": 8080,
                        "protocol": "tcp",
                        "received": "0",
                        "sent": "13882"
                    },
                    {
                        "interface_name": "tun0",
                        "local_ip_address": "192.168.10.87",
                        "local_port": 43254,
                        "remote_ip_address": "192.168.10.34",
                        "remote_port": 5353,
                        "protocol": "udp",
                        "received": "120",
                        "sent": "64"
                    }
                ],
                "open_sockets": [
                    {
                        "ip_address": "0.0.0.0",
                        "port": 68,
                        "protocol": "udp",
                        "process": "dhclient"
                    },
                    {
                        "ip_address": "::",
                        "port": 37863,
                        "protocol": "tcp",
                        "process": "java"
                    }
                ]
            }
        });
        assert_json_eq!(actual, expected);
    }
}
