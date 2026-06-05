use serde::{Deserialize, Serialize};

const SERVICE_NAME: &str = "com.ozmarket.mobile";
const CLAIMS_KEY: &str = "marketplace-claims";

/// Mirrors oz-market-auth-core Claims struct to avoid pulling in JWT deps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Claims {
    pub sub: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub seller_account_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub buyer_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hardware_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
}

/// Store claims in the OS keychain.
pub fn store_claims(claims: &Claims) -> Result<(), String> {
    let json = serde_json::to_string(claims).map_err(|e| e.to_string())?;
    let entry = keyring::Entry::new(SERVICE_NAME, CLAIMS_KEY).map_err(|e| e.to_string())?;
    entry.set_password(&json).map_err(|e| e.to_string())
}

/// Load stored claims from the OS keychain.
pub fn load_claims() -> Result<Claims, String> {
    let entry = keyring::Entry::new(SERVICE_NAME, CLAIMS_KEY).map_err(|e| e.to_string())?;
    let json = match entry.get_password() {
        Ok(v) => v,
        Err(keyring::Error::NoEntry) => return Err("no stored claims".to_string()),
        Err(e) => return Err(e.to_string()),
    };
    serde_json::from_str(&json).map_err(|e| e.to_string())
}

/// Remove stored claims from the OS keychain.
pub fn clear_claims() -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, CLAIMS_KEY).map_err(|e| e.to_string())?;
    entry.delete_credential().map_err(|e| e.to_string())
}
