//! Metrics for EVM anchoring operations.
//!
//! Provides observability into anchor transaction submission, confirmation,
//! gas costs, and wallet balance.

use std::time::Duration;

use ceramic_metrics::{register, Recorder};
use prometheus_client::{
    encoding::EncodeLabelSet,
    metrics::{
        counter::Counter,
        family::Family,
        gauge::Gauge,
        histogram::{exponential_buckets, Histogram},
    },
    registry::Registry,
};

/// Labels for metrics that are chain-specific.
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ChainLabels {
    /// The EVM chain ID (e.g., 1 for Ethereum mainnet, 100 for Gnosis).
    pub chain_id: u64,
}

/// Labels for error metrics.
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ErrorLabels {
    /// The EVM chain ID.
    pub chain_id: u64,
    /// The type of error that occurred.
    pub error_type: String,
}

/// Events for EVM anchoring operations.
#[derive(Clone, Debug)]
pub enum EvmEvent {
    /// A transaction was submitted (before confirmation).
    TransactionSubmitted {
        /// The EVM chain ID.
        chain_id: u64,
    },
    /// A transaction was successfully confirmed.
    TransactionSucceeded {
        /// The EVM chain ID.
        chain_id: u64,
        /// Duration from submission to confirmed receipt.
        transaction_duration: Duration,
        /// Duration spent waiting for confirmations.
        confirmation_duration: Duration,
        /// Gas cost in wei.
        gas_cost_wei: u128,
    },
    /// A transaction failed.
    TransactionFailed {
        /// The EVM chain ID.
        chain_id: u64,
        /// The type of error that occurred.
        error_type: String,
    },
    /// A retry attempt was made.
    RetryAttempt {
        /// The EVM chain ID.
        chain_id: u64,
    },
    /// Wallet balance was updated.
    WalletBalanceUpdated {
        /// The EVM chain ID.
        chain_id: u64,
        /// Current balance in wei.
        balance_wei: u128,
    },
}

/// Metrics for EVM anchoring operations.
#[derive(Clone, Debug)]
pub struct Metrics {
    /// Total anchor transactions submitted.
    transactions_total: Family<ChainLabels, Counter>,
    /// Successfully confirmed anchor transactions.
    transactions_successful: Family<ChainLabels, Counter>,
    /// Failed anchor transactions by error type.
    transactions_failed: Family<ErrorLabels, Counter>,
    /// Total retry attempts.
    retry_attempts: Family<ChainLabels, Counter>,

    /// Current wallet balance in gwei.
    wallet_balance_gwei: Family<ChainLabels, Gauge>,
    /// Unix timestamp of last successful anchor.
    last_success_timestamp: Family<ChainLabels, Gauge>,

    /// Time from submission to confirmed receipt in seconds.
    transaction_duration: Family<ChainLabels, Histogram>,
    /// Time spent waiting for block confirmations in seconds.
    confirmation_duration: Family<ChainLabels, Histogram>,
    /// Gas cost per transaction in gwei.
    gas_cost_gwei: Family<ChainLabels, Histogram>,
}

impl Metrics {
    /// Register and construct Metrics.
    pub fn register(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("anchor_evm");

        register!(
            transactions_total,
            "Total anchor transactions submitted to the EVM chain",
            Family::<ChainLabels, Counter>::default(),
            sub_registry
        );

        register!(
            transactions_successful,
            "Successfully confirmed anchor transactions",
            Family::<ChainLabels, Counter>::default(),
            sub_registry
        );

        register!(
            transactions_failed,
            "Failed anchor transactions by error type",
            Family::<ErrorLabels, Counter>::default(),
            sub_registry
        );

        register!(
            retry_attempts,
            "Total retry attempts for anchor transactions",
            Family::<ChainLabels, Counter>::default(),
            sub_registry
        );

        register!(
            wallet_balance_gwei,
            "Current wallet balance in gwei",
            Family::<ChainLabels, Gauge>::default(),
            sub_registry
        );

        register!(
            last_success_timestamp,
            "Unix timestamp of last successful anchor",
            Family::<ChainLabels, Gauge>::default(),
            sub_registry
        );

        // Transaction duration: 1s to ~17 minutes (1, 2, 4, 8, ... 1024 seconds)
        register!(
            transaction_duration,
            "Time from submission to confirmed receipt in seconds",
            Family::<ChainLabels, Histogram>::new_with_constructor(|| {
                Histogram::new(exponential_buckets(1.0, 2.0, 11))
            }),
            sub_registry
        );

        // Confirmation duration: 1s to ~17 minutes
        register!(
            confirmation_duration,
            "Time spent waiting for block confirmations in seconds",
            Family::<ChainLabels, Histogram>::new_with_constructor(|| {
                Histogram::new(exponential_buckets(1.0, 2.0, 11))
            }),
            sub_registry
        );

        // Gas cost in gwei: 0.001 to 10^9 gwei (covers dust to mass transactions)
        register!(
            gas_cost_gwei,
            "Gas cost per transaction in gwei",
            Family::<ChainLabels, Histogram>::new_with_constructor(|| {
                Histogram::new(exponential_buckets(0.001, 10.0, 13))
            }),
            sub_registry
        );

        Self {
            transactions_total,
            transactions_successful,
            transactions_failed,
            retry_attempts,
            wallet_balance_gwei,
            last_success_timestamp,
            transaction_duration,
            confirmation_duration,
            gas_cost_gwei,
        }
    }
}

impl Recorder<EvmEvent> for Metrics {
    fn record(&self, event: &EvmEvent) {
        match event {
            EvmEvent::TransactionSubmitted { chain_id } => {
                let labels = ChainLabels { chain_id: *chain_id };
                self.transactions_total.get_or_create(&labels).inc();
            }
            EvmEvent::TransactionSucceeded {
                chain_id,
                transaction_duration,
                confirmation_duration,
                gas_cost_wei,
            } => {
                let labels = ChainLabels { chain_id: *chain_id };

                self.transactions_successful.get_or_create(&labels).inc();

                self.transaction_duration
                    .get_or_create(&labels)
                    .observe(transaction_duration.as_secs_f64());

                self.confirmation_duration
                    .get_or_create(&labels)
                    .observe(confirmation_duration.as_secs_f64());

                // Convert gas cost from wei to gwei for histogram
                let gas_gwei = *gas_cost_wei as f64 / 1_000_000_000.0;
                self.gas_cost_gwei.get_or_create(&labels).observe(gas_gwei);

                // Update last success timestamp
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                self.last_success_timestamp.get_or_create(&labels).set(now);
            }
            EvmEvent::TransactionFailed { chain_id, error_type } => {
                let labels = ErrorLabels {
                    chain_id: *chain_id,
                    error_type: error_type.clone(),
                };
                self.transactions_failed.get_or_create(&labels).inc();
            }
            EvmEvent::RetryAttempt { chain_id } => {
                let labels = ChainLabels { chain_id: *chain_id };
                self.retry_attempts.get_or_create(&labels).inc();
            }
            EvmEvent::WalletBalanceUpdated { chain_id, balance_wei } => {
                let labels = ChainLabels { chain_id: *chain_id };
                // Convert wei to gwei (divide by 10^9) to fit in i64
                let balance_gwei = (*balance_wei / 1_000_000_000) as i64;
                self.wallet_balance_gwei
                    .get_or_create(&labels)
                    .set(balance_gwei);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_register() {
        let mut registry = Registry::default();
        let _metrics = Metrics::register(&mut registry);
    }

    #[test]
    fn test_record_transaction_submitted() {
        let mut registry = Registry::default();
        let metrics = Metrics::register(&mut registry);

        metrics.record(&EvmEvent::TransactionSubmitted { chain_id: 100 });
    }

    #[test]
    fn test_record_transaction_succeeded() {
        let mut registry = Registry::default();
        let metrics = Metrics::register(&mut registry);

        metrics.record(&EvmEvent::TransactionSucceeded {
            chain_id: 100,
            transaction_duration: Duration::from_secs(10),
            confirmation_duration: Duration::from_secs(5),
            gas_cost_wei: 1_000_000_000_000, // 1000 gwei
        });
    }

    #[test]
    fn test_record_transaction_failed() {
        let mut registry = Registry::default();
        let metrics = Metrics::register(&mut registry);

        metrics.record(&EvmEvent::TransactionFailed {
            chain_id: 100,
            error_type: "insufficient_funds".to_string(),
        });
    }

    #[test]
    fn test_record_retry_attempt() {
        let mut registry = Registry::default();
        let metrics = Metrics::register(&mut registry);

        metrics.record(&EvmEvent::RetryAttempt { chain_id: 100 });
    }

    #[test]
    fn test_record_wallet_balance() {
        let mut registry = Registry::default();
        let metrics = Metrics::register(&mut registry);

        // 1 ETH in wei
        metrics.record(&EvmEvent::WalletBalanceUpdated {
            chain_id: 100,
            balance_wei: 1_000_000_000_000_000_000,
        });
    }
}
