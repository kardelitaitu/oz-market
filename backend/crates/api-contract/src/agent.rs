use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct AgentQueryRequest {
    pub query: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct AgentAction {
    pub action_type: String,
    pub label: String,
    pub params: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, schemars::JsonSchema)]
pub struct AgentQueryResponse {
    pub message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<AgentAction>,
    pub conversation_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub listing_ids: Option<Vec<String>>,
}
