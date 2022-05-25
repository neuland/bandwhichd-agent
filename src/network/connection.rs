use ::std::fmt;
use ::std::net::IpAddr;

use ::std::net::SocketAddr;

#[derive(PartialEq, Hash, Eq, Clone, PartialOrd, Ord, Debug, Copy)]
pub enum Protocol {
    Tcp,
    Udp,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Protocol::Tcp => write!(f, "tcp"),
            Protocol::Udp => write!(f, "udp"),
        }
    }
}

#[derive(Clone, Ord, PartialOrd, PartialEq, Eq, Hash, Debug, Copy)]
pub struct Socket {
    pub ip: IpAddr,
    pub port: u16,
}

impl From<Socket> for SocketAddr {
    fn from(socket: Socket) -> Self {
        SocketAddr::new(socket.ip, socket.port)
    }
}

#[derive(PartialEq, Hash, Eq, Clone, PartialOrd, Ord, Debug, Copy)]
pub struct LocalSocket {
    pub ip: IpAddr,
    pub port: u16,
    pub protocol: Protocol,
}

impl From<LocalSocket> for SocketAddr {
    fn from(local_socket: LocalSocket) -> Self {
        SocketAddr::new(local_socket.ip, local_socket.port)
    }
}

#[derive(PartialEq, Hash, Eq, Clone, PartialOrd, Ord, Debug, Copy)]
pub struct Connection {
    pub remote_socket: Socket,
    pub local_socket: LocalSocket,
}

impl Connection {
    pub fn new(
        remote_socket: SocketAddr,
        local_ip: IpAddr,
        local_port: u16,
        protocol: Protocol,
    ) -> Self {
        Connection {
            remote_socket: Socket {
                ip: remote_socket.ip(),
                port: remote_socket.port(),
            },
            local_socket: LocalSocket {
                ip: local_ip,
                port: local_port,
                protocol,
            },
        }
    }
}
