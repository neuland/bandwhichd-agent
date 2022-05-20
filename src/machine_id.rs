use uuid::Uuid;

#[derive(Clone)]
pub struct MachineId {
    secure_uuid: Uuid,
}

impl MachineId {
    pub fn new(raw_value: String) -> Self {
        let secure_uuid = calculate_secure_uuid(&raw_value);
        MachineId { secure_uuid }
    }

    pub fn secure_uuid(&self) -> Uuid {
        self.secure_uuid
    }
}

impl Default for MachineId {
    fn default() -> Self {
        MachineId::new(
            std::fs::read_to_string("/etc/machine-id").expect("unable to read /etc/machine-id"),
        )
    }
}

fn calculate_secure_uuid(raw_value: &str) -> Uuid {
    use sha3::{Digest, Sha3_256};
    let mut hasher = Sha3_256::new();
    hasher.update(raw_value);
    let output = hasher.finalize();
    let slice = output.as_slice();
    Uuid::from_slice(&slice[0..16]).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_have_secure_uuid() {
        // given
        let machine_id = MachineId::new("<machine-id>".to_string());

        // when
        let result = machine_id.secure_uuid();

        // then
        assert_eq!(result, calculate_secure_uuid("<machine-id>"));
    }
}
