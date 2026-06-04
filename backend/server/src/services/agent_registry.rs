use dashmap::DashMap;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentMetadata {
    pub id: Uuid,
    pub endpoint: String,
    pub capabilities: Vec<String>,
    pub is_active: bool,
}

pub struct AgentRegistry {
    agents: DashMap<Uuid, AgentMetadata>,
}

impl AgentRegistry {
    pub fn new() -> Self {
        Self {
            agents: DashMap::new(),
        }
    }

    pub fn register_agent(&self, agent: AgentMetadata) {
        self.agents.insert(agent.id, agent);
    }

    pub fn deregister_agent(&self, agent_id: &Uuid) -> Option<AgentMetadata> {
        self.agents.remove(agent_id).map(|(_, v)| v)
    }

    pub fn get_agent(&self, agent_id: &Uuid) -> Option<AgentMetadata> {
        self.agents.get(agent_id).map(|r| r.value().clone())
    }

    pub fn get_matching_agents(&self, capabilities: &[String]) -> Vec<AgentMetadata> {
        self.agents
            .iter()
            .filter(|r| {
                let val = r.value();
                val.is_active && capabilities.iter().all(|c| val.capabilities.contains(c))
            })
            .map(|r| r.value().clone())
            .collect()
    }

    pub fn list_agents(&self) -> Vec<AgentMetadata> {
        self.agents.iter().map(|r| r.value().clone()).collect()
    }

    pub fn agent_count(&self) -> usize {
        self.agents.len()
    }
}

impl Default for AgentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn make_agent(id: Uuid, capabilities: Vec<String>, is_active: bool) -> AgentMetadata {
        AgentMetadata {
            id,
            endpoint: format!("http://agent-{}.local:8080", id),
            capabilities,
            is_active,
        }
    }

    #[test]
    fn register_and_retrieve_agent() {
        let registry = AgentRegistry::new();
        let id = Uuid::new_v4();
        let agent = make_agent(id, vec!["search".into()], true);

        registry.register_agent(agent.clone());

        let retrieved = registry.get_agent(&id).expect("agent should exist");
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.endpoint, agent.endpoint);
        assert!(retrieved.is_active);
    }

    #[test]
    fn deregister_agent() {
        let registry = AgentRegistry::new();
        let id = Uuid::new_v4();
        let agent = make_agent(id, vec!["search".into()], true);

        registry.register_agent(agent);
        let removed = registry.deregister_agent(&id);
        assert!(removed.is_some());
        assert!(registry.get_agent(&id).is_none());
    }

    #[test]
    fn deregister_nonexistent_agent() {
        let registry = AgentRegistry::new();
        let id = Uuid::new_v4();
        assert!(registry.deregister_agent(&id).is_none());
    }

    #[test]
    fn get_matching_agents_filters_by_capability() {
        let registry = AgentRegistry::new();
        let search_id = Uuid::new_v4();
        let chat_id = Uuid::new_v4();
        let full_id = Uuid::new_v4();

        registry.register_agent(make_agent(search_id, vec!["search".into()], true));
        registry.register_agent(make_agent(chat_id, vec!["chat".into()], true));
        registry.register_agent(make_agent(
            full_id,
            vec!["search".into(), "chat".into()],
            true,
        ));

        let search_agents = registry.get_matching_agents(&["search".to_string()]);
        assert_eq!(search_agents.len(), 2);

        let both = registry.get_matching_agents(&["search".to_string(), "chat".to_string()]);
        assert_eq!(both.len(), 1);
        assert_eq!(both[0].id, full_id);
    }

    #[test]
    fn get_matching_agents_excludes_inactive() {
        let registry = AgentRegistry::new();
        let id = Uuid::new_v4();
        registry.register_agent(make_agent(id, vec!["search".into()], false));

        let agents = registry.get_matching_agents(&["search".to_string()]);
        assert!(agents.is_empty());
    }

    #[test]
    fn list_agents_returns_all() {
        let registry = AgentRegistry::new();
        let id_a = Uuid::new_v4();
        let id_b = Uuid::new_v4();
        registry.register_agent(make_agent(id_a, vec![], true));
        registry.register_agent(make_agent(id_b, vec![], true));

        assert_eq!(registry.list_agents().len(), 2);
        assert_eq!(registry.agent_count(), 2);
    }

    #[test]
    fn concurrent_registration() {
        let registry = Arc::new(AgentRegistry::new());
        let mut handles = Vec::new();

        for i in 0..10 {
            let r = Arc::clone(&registry);
            handles.push(std::thread::spawn(move || {
                let id = Uuid::new_v4();
                r.register_agent(make_agent(id, vec![format!("cap_{}", i)], true));
            }));
        }

        for h in handles {
            h.join().expect("thread panicked");
        }

        assert_eq!(registry.agent_count(), 10);
    }
}
