//! Metrics for anchor service operations.
//!
//! Provides observability into anchor batch processing, request handling,
//! Merkle tree building, and time event storage.

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

/// Labels for error metrics.
#[derive(Clone, Debug, Hash, PartialEq, Eq, EncodeLabelSet)]
pub struct ErrorLabels {
    /// The type of error that occurred.
    pub error_type: String,
}

/// Events for anchor service operations.
#[derive(Clone, Debug)]
pub enum AnchorEvent {
    /// The anchor service started running.
    ServiceStarted,
    /// The anchor service stopped.
    ServiceStopped,
    /// A batch processing attempt started.
    BatchStarted,
    /// A batch was successfully processed.
    BatchSucceeded {
        /// Number of requests fetched from store.
        requests_received: u64,
        /// Number of requests after deduplication.
        requests_after_dedup: u64,
        /// Number of time events created and stored.
        time_events_stored: u64,
        /// Total duration to process the batch.
        total_duration: Duration,
        /// Duration to fetch requests from store.
        fetch_duration: Duration,
        /// Duration to build the Merkle tree.
        tree_build_duration: Duration,
        /// Duration to anchor (call TransactionManager).
        anchor_duration: Duration,
        /// Duration to store time events.
        store_duration: Duration,
    },
    /// A batch failed.
    BatchFailed {
        /// The type of error that occurred.
        error_type: String,
    },
    /// A batch was skipped due to no pending requests.
    BatchEmpty,
    /// The high water mark was updated.
    HighWaterMarkUpdated {
        /// The new high water mark value.
        value: i64,
    },
}

/// Metrics for anchor service operations.
#[derive(Clone, Debug)]
pub struct Metrics {
    // Counters
    /// Total anchor batches attempted.
    batches_total: Counter,
    /// Successfully completed batches.
    batches_successful: Counter,
    /// Failed batches by error type.
    batches_failed: Family<ErrorLabels, Counter>,
    /// Batches skipped due to no pending requests.
    batches_empty: Counter,
    /// Total requests fetched from store.
    requests_received: Counter,
    /// Requests remaining after deduplication.
    requests_deduplicated: Counter,
    /// Time events successfully stored.
    time_events_stored: Counter,

    // Gauges
    /// Current high water mark position.
    high_water_mark: Gauge,
    /// 1 if service is running, 0 if stopped.
    service_running: Gauge,

    // Histograms
    /// Number of requests per batch.
    batch_size: Histogram,
    /// Total time to process a batch end-to-end.
    batch_duration: Histogram,
    /// Time to fetch requests from store.
    fetch_duration: Histogram,
    /// Time to build the Merkle tree.
    tree_build_duration: Histogram,
    /// Time to anchor via TransactionManager.
    anchor_duration: Histogram,
    /// Time to store time events.
    store_duration: Histogram,
    /// Ratio of deduplicated to original requests (0.0-1.0).
    dedup_ratio: Histogram,
}

impl Metrics {
    /// Register and construct Metrics.
    pub fn register(registry: &mut Registry) -> Self {
        let sub_registry = registry.sub_registry_with_prefix("anchor_service");

        register!(
            batches_total,
            "Total anchor batches attempted",
            Counter::default(),
            sub_registry
        );

        register!(
            batches_successful,
            "Successfully completed anchor batches",
            Counter::default(),
            sub_registry
        );

        register!(
            batches_failed,
            "Failed anchor batches by error type",
            Family::<ErrorLabels, Counter>::default(),
            sub_registry
        );

        register!(
            batches_empty,
            "Anchor batches skipped due to no pending requests",
            Counter::default(),
            sub_registry
        );

        register!(
            requests_received,
            "Total anchor requests fetched from store",
            Counter::default(),
            sub_registry
        );

        register!(
            requests_deduplicated,
            "Anchor requests remaining after deduplication",
            Counter::default(),
            sub_registry
        );

        register!(
            time_events_stored,
            "Time events successfully stored",
            Counter::default(),
            sub_registry
        );

        register!(
            high_water_mark,
            "Current high water mark position",
            Gauge::default(),
            sub_registry
        );

        register!(
            service_running,
            "Whether the anchor service is running (1) or stopped (0)",
            Gauge::default(),
            sub_registry
        );

        // Batch size: 1 to ~1M events (1, 2, 4, 8, ... 1048576)
        register!(
            batch_size,
            "Number of anchor requests per batch",
            Histogram::new(exponential_buckets(1.0, 2.0, 21)),
            sub_registry
        );

        // Batch duration: 100ms to ~1.8 hours (0.1, 0.2, 0.4, ... 6553.6 seconds)
        register!(
            batch_duration,
            "Total time to process an anchor batch in seconds",
            Histogram::new(exponential_buckets(0.1, 2.0, 17)),
            sub_registry
        );

        // Fetch duration: 1ms to ~16 seconds
        register!(
            fetch_duration,
            "Time to fetch anchor requests from store in seconds",
            Histogram::new(exponential_buckets(0.001, 2.0, 15)),
            sub_registry
        );

        // Tree build duration: 1ms to ~16 seconds
        register!(
            tree_build_duration,
            "Time to build Merkle tree in seconds",
            Histogram::new(exponential_buckets(0.001, 2.0, 15)),
            sub_registry
        );

        // Anchor duration: 1s to ~2.3 hours (transaction + confirmations)
        register!(
            anchor_duration,
            "Time to anchor via TransactionManager in seconds",
            Histogram::new(exponential_buckets(1.0, 2.0, 14)),
            sub_registry
        );

        // Store duration: 1ms to ~16 seconds
        register!(
            store_duration,
            "Time to store time events in seconds",
            Histogram::new(exponential_buckets(0.001, 2.0, 15)),
            sub_registry
        );

        // Dedup ratio: 0.0 to 1.0 in 10 buckets
        register!(
            dedup_ratio,
            "Ratio of requests remaining after deduplication (0.0-1.0)",
            Histogram::new(
                [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0].into_iter()
            ),
            sub_registry
        );

        Self {
            batches_total,
            batches_successful,
            batches_failed,
            batches_empty,
            requests_received,
            requests_deduplicated,
            time_events_stored,
            high_water_mark,
            service_running,
            batch_size,
            batch_duration,
            fetch_duration,
            tree_build_duration,
            anchor_duration,
            store_duration,
            dedup_ratio,
        }
    }
}

impl Recorder<AnchorEvent> for Metrics {
    fn record(&self, event: &AnchorEvent) {
        match event {
            AnchorEvent::ServiceStarted => {
                self.service_running.set(1);
            }
            AnchorEvent::ServiceStopped => {
                self.service_running.set(0);
            }
            AnchorEvent::BatchStarted => {
                self.batches_total.inc();
            }
            AnchorEvent::BatchSucceeded {
                requests_received,
                requests_after_dedup,
                time_events_stored,
                total_duration,
                fetch_duration,
                tree_build_duration,
                anchor_duration,
                store_duration,
            } => {
                self.batches_successful.inc();
                self.requests_received.inc_by(*requests_received);
                self.requests_deduplicated.inc_by(*requests_after_dedup);
                self.time_events_stored.inc_by(*time_events_stored);

                self.batch_size.observe(*requests_after_dedup as f64);
                self.batch_duration.observe(total_duration.as_secs_f64());
                self.fetch_duration.observe(fetch_duration.as_secs_f64());
                self.tree_build_duration
                    .observe(tree_build_duration.as_secs_f64());
                self.anchor_duration.observe(anchor_duration.as_secs_f64());
                self.store_duration.observe(store_duration.as_secs_f64());

                // Calculate dedup ratio (how many requests survived deduplication)
                if *requests_received > 0 {
                    let ratio = *requests_after_dedup as f64 / *requests_received as f64;
                    self.dedup_ratio.observe(ratio);
                }
            }
            AnchorEvent::BatchFailed { error_type } => {
                let labels = ErrorLabels {
                    error_type: error_type.clone(),
                };
                self.batches_failed.get_or_create(&labels).inc();
            }
            AnchorEvent::BatchEmpty => {
                self.batches_empty.inc();
            }
            AnchorEvent::HighWaterMarkUpdated { value } => {
                self.high_water_mark.set(*value);
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
    fn test_record_service_lifecycle() {
        let mut registry = Registry::default();
        let metrics = Metrics::register(&mut registry);

        metrics.record(&AnchorEvent::ServiceStarted);
        metrics.record(&AnchorEvent::ServiceStopped);
    }

    #[test]
    fn test_record_batch_succeeded() {
        let mut registry = Registry::default();
        let metrics = Metrics::register(&mut registry);

        metrics.record(&AnchorEvent::BatchSucceeded {
            requests_received: 100,
            requests_after_dedup: 80,
            time_events_stored: 80,
            total_duration: Duration::from_secs(30),
            fetch_duration: Duration::from_millis(100),
            tree_build_duration: Duration::from_millis(50),
            anchor_duration: Duration::from_secs(25),
            store_duration: Duration::from_millis(200),
        });
    }

    #[test]
    fn test_record_batch_failed() {
        let mut registry = Registry::default();
        let metrics = Metrics::register(&mut registry);

        metrics.record(&AnchorEvent::BatchFailed {
            error_type: "anchor_error".to_string(),
        });
    }

    #[test]
    fn test_record_batch_empty() {
        let mut registry = Registry::default();
        let metrics = Metrics::register(&mut registry);

        metrics.record(&AnchorEvent::BatchEmpty);
    }

    #[test]
    fn test_record_high_water_mark() {
        let mut registry = Registry::default();
        let metrics = Metrics::register(&mut registry);

        metrics.record(&AnchorEvent::HighWaterMarkUpdated { value: 12345 });
    }
}
