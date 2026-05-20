use std::fmt;

#[derive(Debug)]
pub enum ApiError {
    Http(reqwest::Error),
    Status(u16, String),
    Serde(serde_json::Error),
}

impl ApiError {
    pub async fn from_response(resp: reqwest::Response) -> Self {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        ApiError::Status(status, body)
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApiError::Http(e) => write!(f, "HTTP error: {e}"),
            ApiError::Status(code, body) => {
                write!(f, "API error ({}): {}", code, body)
            }
            ApiError::Serde(e) => write!(f, "Serialization error: {e}"),
        }
    }
}

impl From<reqwest::Error> for ApiError {
    fn from(e: reqwest::Error) -> Self {
        ApiError::Http(e)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::Serde(e)
    }
}

impl From<ApiError> for String {
    fn from(e: ApiError) -> Self {
        e.to_string()
    }
}
