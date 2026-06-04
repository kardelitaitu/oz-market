use async_trait::async_trait;
use rust_decimal::Decimal;
use std::fmt::{Display, Formatter};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionType {
    Deposit,
    Spend,
    Refund,
    Adjustment,
}

impl TransactionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TransactionType::Deposit => "deposit",
            TransactionType::Spend => "spend",
            TransactionType::Refund => "refund",
            TransactionType::Adjustment => "adjustment",
        }
    }
}

impl std::str::FromStr for TransactionType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "deposit" => Ok(TransactionType::Deposit),
            "spend" => Ok(TransactionType::Spend),
            "refund" => Ok(TransactionType::Refund),
            "adjustment" => Ok(TransactionType::Adjustment),
            _ => Err(format!("invalid transaction type: {s}")),
        }
    }
}

impl Display for TransactionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreditAccount {
    pub agent_id: String,
    pub balance_credits: Decimal,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreditTransaction {
    pub id: Uuid,
    pub agent_id: String,
    pub amount: Decimal,
    pub transaction_type: TransactionType,
    pub idempotency_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewTransaction {
    pub id: Uuid,
    pub agent_id: String,
    pub amount: Decimal,
    pub tx_type: TransactionType,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CreditLedgerError {
    AgentNotFound(String),
    InsufficientCredits {
        requested: Decimal,
        available: Decimal,
    },
    DuplicateIdempotencyKey(String),
    DatabaseError(String),
}

impl Display for CreditLedgerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            CreditLedgerError::AgentNotFound(id) => {
                write!(f, "Agent not found: {id}")
            }
            CreditLedgerError::InsufficientCredits {
                requested,
                available,
            } => {
                write!(
                    f,
                    "Insufficient credits: requested {requested}, available {available}"
                )
            }
            CreditLedgerError::DuplicateIdempotencyKey(key) => {
                write!(f, "Transaction with idempotency key '{key}' already exists")
            }
            CreditLedgerError::DatabaseError(msg) => {
                write!(f, "Database error: {msg}")
            }
        }
    }
}

impl std::error::Error for CreditLedgerError {}

#[async_trait]
pub trait CreditLedgerRepository: Send + Sync {
    async fn get_balance(&self, agent_id: &str) -> Result<CreditAccount, CreditLedgerError>;

    async fn apply_transaction(
        &self,
        tx: &NewTransaction,
    ) -> Result<CreditAccount, CreditLedgerError>;

    async fn get_transaction_history(
        &self,
        agent_id: &str,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<CreditTransaction>, CreditLedgerError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_type_as_str_roundtrip() {
        let cases = [
            (TransactionType::Deposit, "deposit"),
            (TransactionType::Spend, "spend"),
            (TransactionType::Refund, "refund"),
            (TransactionType::Adjustment, "adjustment"),
        ];
        for (variant, expected) in &cases {
            assert_eq!(variant.as_str(), *expected);
            let parsed: TransactionType = expected.parse().unwrap();
            assert_eq!(parsed, *variant);
        }
    }

    #[test]
    fn transaction_type_from_str_invalid() {
        assert!("unknown".parse::<TransactionType>().is_err());
        assert!("".parse::<TransactionType>().is_err());
        assert!("DEPOSIT".parse::<TransactionType>().is_err());
    }

    #[test]
    fn transaction_type_display_matches_as_str() {
        let cases = [
            TransactionType::Deposit,
            TransactionType::Spend,
            TransactionType::Refund,
            TransactionType::Adjustment,
        ];
        for variant in &cases {
            assert_eq!(format!("{variant}"), variant.as_str());
        }
    }

    #[test]
    fn credit_ledger_error_agent_not_found() {
        let err = CreditLedgerError::AgentNotFound("agent-1".into());
        assert_eq!(err.to_string(), "Agent not found: agent-1");
    }

    #[test]
    fn credit_ledger_error_insufficient_credits() {
        let err = CreditLedgerError::InsufficientCredits {
            requested: Decimal::new(100, 0),
            available: Decimal::new(50, 0),
        };
        assert_eq!(
            err.to_string(),
            "Insufficient credits: requested 100, available 50"
        );
    }

    #[test]
    fn credit_ledger_error_duplicate_idempotency_key() {
        let err = CreditLedgerError::DuplicateIdempotencyKey("key-123".into());
        assert_eq!(
            err.to_string(),
            "Transaction with idempotency key 'key-123' already exists"
        );
    }

    #[test]
    fn credit_ledger_error_database_error() {
        let err = CreditLedgerError::DatabaseError("connection refused".into());
        assert_eq!(err.to_string(), "Database error: connection refused");
    }

    #[test]
    fn credit_ledger_error_implements_std_error() {
        let err = CreditLedgerError::AgentNotFound("x".into());
        let std_err: &dyn std::error::Error = &err;
        assert_eq!(std_err.to_string(), "Agent not found: x");
    }

    #[test]
    fn new_transaction_struct_fields() {
        let id = Uuid::new_v4();
        let tx = NewTransaction {
            id,
            agent_id: "agent-1".into(),
            amount: Decimal::new(1000, 4),
            tx_type: TransactionType::Deposit,
            idempotency_key: "key-1".into(),
        };
        assert_eq!(tx.id, id);
        assert_eq!(tx.agent_id, "agent-1");
        assert_eq!(tx.amount, Decimal::new(1000, 4));
        assert_eq!(tx.tx_type, TransactionType::Deposit);
        assert_eq!(tx.idempotency_key, "key-1");
    }

    #[test]
    fn decimal_max_numeric_precision_boundary() {
        use std::str::FromStr;
        // NUMERIC(20,4) accepts up to 9999999999999999.9999 (16 digits + 4 decimal places).
        let max = Decimal::from_str("9999999999999999.9999").expect("must parse max NUMERIC(20,4)");
        let tx = NewTransaction {
            id: Uuid::new_v4(),
            agent_id: "agent-max".into(),
            amount: max,
            tx_type: TransactionType::Deposit,
            idempotency_key: "max-key".into(),
        };
        assert_eq!(tx.amount, max);
        assert_eq!(tx.amount.scale(), 4);
        assert_eq!(tx.amount.to_string(), "9999999999999999.9999");
    }

    #[test]
    fn decimal_amount_serialization_preserves_scale() {
        use std::str::FromStr;
        // Strings with explicit scale and integers should produce identical Decimal values.
        let from_string = Decimal::from_str("100.0000").unwrap();
        let from_int = Decimal::new(1_000_000, 4); // 100.0000
        assert_eq!(from_string, from_int);
        // Scale precision survives arithmetic.
        let summed = from_string + from_int;
        assert_eq!(summed, Decimal::from_str("200.0000").unwrap());
        // String formatting preserves trailing zeros up to declared scale.
        assert_eq!(format!("{summed:.4}"), "200.0000");
    }
}
