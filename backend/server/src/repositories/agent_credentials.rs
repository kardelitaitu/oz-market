use crate::models::db::AgentCredentialRow;
use crate::repositories::{RepositoryError, RepositoryErrorKind};

#[async_trait::async_trait]
pub trait AgentCredentialRepository: Send + Sync {
    async fn get_by_subject(
        &self,
        subject: &str,
    ) -> Result<Option<AgentCredentialRow>, RepositoryError>;
}

pub fn not_found(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(RepositoryErrorKind::NotFound, message)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_not_found_error() {
        let error = not_found("agent not found");
        assert_eq!(error.kind, RepositoryErrorKind::NotFound);
        assert_eq!(error.message, "agent not found");
    }
}
