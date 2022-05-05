use crate::network::Protocol;
use crate::{OpenSockets, Utilization};
use serde::{Serialize, Serializer};
use std::net::IpAddr;
use std::time::SystemTime;

#[derive(Serialize)]
#[serde(tag = "version", content = "message")]
pub enum VersionedMessage {
    #[serde(rename = "1")]
    V1(MessageV1),
}

impl VersionedMessage {
    pub fn from(agent_name: String, utilization: Utilization, open_sockets: OpenSockets) -> Self {
        VersionedMessage::V1(MessageV1::from(agent_name, utilization, open_sockets))
    }
}

#[derive(Serialize)]
pub struct MessageV1 {
    pub agent_name: String,
    pub start: TimestampV1,
    pub stop: TimestampV1,
    pub connections: Vec<ConnectionV1>,
    pub open_sockets: Vec<OpenSocketV1>,
}

impl MessageV1 {
    pub fn from(agent_name: String, utilization: Utilization, open_sockets: OpenSockets) -> Self {
        MessageV1 {
            agent_name,
            start: TimestampV1(utilization.start),
            stop: TimestampV1(utilization.stop),
            connections: utilization
                .connections
                .into_iter()
                .map(|(connection, connection_info)| ConnectionV1 {
                    interface_name: connection_info.interface_name,
                    protocol: ProtocolV1(connection.local_socket.protocol),
                    local_ip_address: IpAddressV1(connection.local_socket.ip),
                    local_port: connection.local_socket.port,
                    remote_ip_address: IpAddressV1(connection.remote_socket.ip),
                    remote_port: connection.remote_socket.port,
                    received: connection_info.total_bytes_downloaded,
                    sent: connection_info.total_bytes_uploaded,
                })
                .collect(),
            open_sockets: open_sockets
                .sockets_to_procs
                .into_iter()
                .map(|(socket, process)| OpenSocketV1 {
                    ip_address: IpAddressV1(socket.ip),
                    port: socket.port,
                    protocol: ProtocolV1(socket.protocol),
                    process,
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
pub struct ConnectionV1 {
    pub interface_name: String,
    pub protocol: ProtocolV1,
    pub local_ip_address: IpAddressV1,
    pub local_port: u16,
    pub remote_ip_address: IpAddressV1,
    pub remote_port: u16,
    pub received: u128,
    pub sent: u128,
}

#[derive(Serialize)]
pub struct OpenSocketV1 {
    pub ip_address: IpAddressV1,
    pub port: u16,
    pub protocol: ProtocolV1,
    pub process: String,
}

pub struct TimestampV1(SystemTime);

impl Serialize for TimestampV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        time::serde::rfc3339::serialize(&self.0.into(), serializer)
    }
}

pub struct IpAddressV1(IpAddr);

impl Serialize for IpAddressV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(self.0.to_string().as_bytes())
    }
}

pub struct ProtocolV1(Protocol);

impl Serialize for ProtocolV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(
            match self.0 {
                Protocol::Tcp => "tcp",
                Protocol::Udp => "udp",
            }
            .as_ref(),
        )
    }
}
