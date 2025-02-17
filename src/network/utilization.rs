use crate::network::{Connection, Direction, Segment};

use ::std::collections::HashMap;
use std::time::SystemTime;

#[derive(Clone)]
pub struct ConnectionInfo {
    pub interface_name: String,
    pub total_bytes_downloaded: u128,
    pub total_bytes_uploaded: u128,
}

#[derive(Clone)]
pub struct Utilization {
    pub connections: HashMap<Connection, ConnectionInfo>,
    pub start: SystemTime,
    pub stop: SystemTime,
}

impl Utilization {
    pub fn new() -> Self {
        let connections = HashMap::new();
        let now = SystemTime::now();
        Utilization {
            connections,
            start: now,
            stop: now,
        }
    }
    pub fn clone_and_reset(&mut self) -> Self {
        self.stop = SystemTime::now();
        let clone = self.clone();
        self.connections.clear();
        self.start = self.stop;
        clone
    }
    pub fn update(&mut self, seg: Segment) {
        let total_bandwidth = self
            .connections
            .entry(seg.connection)
            .or_insert(ConnectionInfo {
                interface_name: seg.interface_name,
                total_bytes_downloaded: 0,
                total_bytes_uploaded: 0,
            });
        match seg.direction {
            Direction::Download => {
                total_bandwidth.total_bytes_downloaded += seg.data_length;
            }
            Direction::Upload => {
                total_bandwidth.total_bytes_uploaded += seg.data_length;
            }
        }
    }
}
