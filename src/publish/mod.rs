use crate::network::Protocol;
use crate::{OpenSockets, Utilization};
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "version", content = "message")]
pub enum VersionedPublishMessage {
    #[serde(rename = "1")]
    V1(PublishMessageV1),
}

impl VersionedPublishMessage {
    pub fn from(utilization: Utilization, open_sockets: OpenSockets) -> Self {
        VersionedPublishMessage::V1(PublishMessageV1 {
            connections: utilization
                .connections
                .into_iter()
                .map(|(connection, connection_info)| ConnectionV1 {
                    interface_name: connection_info.interface_name,
                    protocol: connection.local_socket.protocol.into(),
                    local_host: connection.local_socket.ip.to_string(),
                    local_port: connection.local_socket.port,
                    process: open_sockets
                        .sockets_to_procs
                        .get(&connection.local_socket)
                        .cloned(),
                    remote_host: connection.remote_socket.ip.to_string(),
                    remote_port: connection.remote_socket.port,
                    received: connection_info.total_bytes_downloaded,
                    sent: connection_info.total_bytes_uploaded,
                })
                .collect(),
        })
    }
}

#[derive(Serialize)]
pub struct PublishMessageV1 {
    pub connections: Vec<ConnectionV1>,
}

#[derive(Serialize)]
pub struct ConnectionV1 {
    pub interface_name: String,
    pub protocol: String,
    pub local_host: String,
    pub local_port: u16,
    pub process: Option<String>,
    pub remote_host: String,
    pub remote_port: u16,
    pub received: u128,
    pub sent: u128,
}

impl From<Protocol> for String {
    fn from(protocol: Protocol) -> Self {
        match protocol {
            Protocol::Tcp => "tcp",
            Protocol::Udp => "udp",
        }
        .to_string()
    }
}
