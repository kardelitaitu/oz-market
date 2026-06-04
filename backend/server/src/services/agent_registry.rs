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

    #[test]
    fn register_overwrites_existing_agent_with_same_id() {
        let registry = AgentRegistry::new();
        let id = Uuid::new_v4();

        registry.register_agent(make_agent(id, vec!["old".into()], true));
        registry.register_agent(make_agent(id, vec!["new".into(), "search".into()], false));

        let stored = registry.get_agent(&id).expect("agent should still exist");
        assert_eq!(
            stored.capabilities,
            vec!["new".to_string(), "search".to_string()]
        );
        assert!(
            !stored.is_active,
            "re-registration must overwrite the old record"
        );
        assert_eq!(registry.agent_count(), 1);
    }

    #[test]
    fn list_agents_includes_inactive_agents() {
        let registry = AgentRegistry::new();
        let active = Uuid::new_v4();
        let inactive = Uuid::new_v4();
        registry.register_agent(make_agent(active, vec![], true));
        registry.register_agent(make_agent(inactive, vec![], false));

        let all = registry.list_agents();
        assert_eq!(all.len(), 2);
        assert!(
            all.iter().any(|a| a.id == active && a.is_active),
            "active agent should appear with is_active=true"
        );
        assert!(
            all.iter().any(|a| a.id == inactive && !a.is_active),
            "inactive agent should still appear in list_agents"
        );
    }

    #[test]
    fn agent_count_zero_after_deregister_all() {
        let registry = AgentRegistry::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        registry.register_agent(make_agent(a, vec![], true));
        registry.register_agent(make_agent(b, vec![], true));
        assert_eq!(registry.agent_count(), 2);

        registry.deregister_agent(&a);
        registry.deregister_agent(&b);
        assert_eq!(registry.agent_count(), 0);
        assert!(registry.list_agents().is_empty());
    }

    #[test]
    fn get_matching_agents_with_empty_query_capabilities_returns_all_active() {
        let registry = AgentRegistry::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        registry.register_agent(make_agent(a, vec!["search".into()], true));
        registry.register_agent(make_agent(b, vec!["chat".into()], true));
        registry.register_agent(make_agent(c, vec!["search".into()], false));

        let matched = registry.get_matching_agents(&[]);
        assert_eq!(
            matched.len(),
            2,
            "empty query capabilities means 'no constraint' so all active agents match"
        );
    }
}
