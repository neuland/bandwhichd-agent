use uuid::Uuid;

#[derive(Clone)]
pub struct AgentId {
    raw_value: Uuid,
}

impl AgentId {
    pub fn new(raw_value: Uuid) -> Self {
        AgentId { raw_value }
    }

    pub fn raw_value(&self) -> Uuid {
        self.raw_value
    }
}

impl Default for AgentId {
    fn default() -> Self {
        AgentId::new(Uuid::new_v4())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::uuid;

    #[test]
    fn should_have_raw_value() {
        // given
        let raw_value = uuid!("5d58f6c1-16c1-4b69-871b-0732e8bd301d");
        let agent_id = AgentId::new(raw_value);

        // when
        let result = agent_id.raw_value();

        // then
        assert_eq!(result, raw_value);
    }
}
