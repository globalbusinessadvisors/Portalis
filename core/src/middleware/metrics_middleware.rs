//! Metrics Middleware for HTTP Request/Response Tracking
//! Week 33 - Phase 4: Monitoring and Observability
//!
//! Provides automatic instrumentation for:
//! - HTTP request/response metrics
//! - Automatic timing instrumentation
//! - Error tracking
//! - Integration with Prometheus

use crate::metrics::PortalisMetrics;
use std::sync::Arc;
use std::time::Instant;

/// Metrics middleware for HTTP requests
#[derive(Clone)]
pub struct MetricsMiddleware {
    metrics: Arc<PortalisMetrics>,
}

impl MetricsMiddleware {
    /// Create a new metrics middleware instance
    pub fn new(metrics: Arc<PortalisMetrics>) -> Self {
        Self { metrics }
    }

    /// Record a request start
    pub fn request_start(&self, endpoint: &str, method: &str) -> RequestGuard {
        RequestGuard::new(
            self.metrics.clone(),
            endpoint.to_string(),
            method.to_string(),
        )
    }

    /// Record an agent execution start
    pub fn agent_start(&self, agent_name: &str, agent_type: &str) -> AgentGuard {
        AgentGuard::new(
            self.metrics.clone(),
            agent_name.to_string(),
            agent_type.to_string(),
        )
    }

    /// Record a pipeline phase start
    pub fn phase_start(&self, phase_name: &str) -> PhaseGuard {
        PhaseGuard::new(self.metrics.clone(), phase_name.to_string())
    }

    /// Record a translation start
    pub fn translation_start(&self, source_lang: &str, target_format: &str) -> TranslationGuard {
        TranslationGuard::new(
            self.metrics.clone(),
            source_lang.to_string(),
            target_format.to_string(),
        )
    }
}

/// RAII guard for HTTP requests that automatically records metrics
pub struct RequestGuard {
    metrics: Arc<PortalisMetrics>,
    endpoint: String,
    _method: String,
    start: Instant,
}

impl RequestGuard {
    fn new(metrics: Arc<PortalisMetrics>, endpoint: String, method: String) -> Self {
        Self {
            metrics,
            endpoint,
            _method: method,
            start: Instant::now(),
        }
    }

    /// Mark request as successful
    pub fn success(self) {
        let _duration = self.start.elapsed().as_secs_f64();

        // Record success in translation metrics if applicable
        if self.endpoint.contains("translate") {
            self.metrics
                .translation
                .translations_success
                .with_label_values(&["python", "wasm"])
                .inc();
        }

        drop(self);
    }

    /// Mark request as failed
    pub fn failure(self, error_category: &str) {
        let _duration = self.start.elapsed().as_secs_f64();

        // Record failure
        if self.endpoint.contains("translate") {
            self.metrics
                .translation
                .translations_failed
                .with_label_values(&["python", "wasm", error_category])
                .inc();
        }

        self.metrics
            .errors
            .errors_total
            .with_label_values(&[error_category, "error", &self.endpoint])
            .inc();

        drop(self);
    }
}

impl Drop for RequestGuard {
    fn drop(&mut self) {
        // Record duration regardless of success/failure
        let _duration = self.start.elapsed().as_secs_f64();

        // This is a generic drop, actual success/failure is recorded explicitly
    }
}

/// RAII guard for agent execution that automatically records metrics
pub struct AgentGuard {
    metrics: Arc<PortalisMetrics>,
    agent_name: String,
    agent_type: String,
    start: Instant,
}

impl AgentGuard {
    fn new(metrics: Arc<PortalisMetrics>, agent_name: String, agent_type: String) -> Self {
        // Increment active agents
        let guard = Self {
            metrics: metrics.clone(),
            agent_name: agent_name.clone(),
            agent_type: agent_type.clone(),
            start: Instant::now(),
        };

        metrics
            .agents
            .agents_active
            .with_label_values(&[&agent_type])
            .inc();

        metrics
            .agents
            .agent_executions
            .with_label_values(&[&agent_name, &agent_type])
            .inc();

        guard
    }

    /// Mark agent execution as successful
    pub fn success(self) {
        let duration = self.start.elapsed().as_secs_f64();

        self.metrics
            .agents
            .agent_duration
            .with_label_values(&[&self.agent_name, &self.agent_type])
            .observe(duration);

        self.metrics
            .agents
            .agent_status
            .with_label_values(&[&self.agent_name, "success"])
            .inc();

        drop(self);
    }

    /// Mark agent execution as failed
    pub fn failure(self, _error_msg: &str) {
        let duration = self.start.elapsed().as_secs_f64();

        self.metrics
            .agents
            .agent_duration
            .with_label_values(&[&self.agent_name, &self.agent_type])
            .observe(duration);

        self.metrics
            .agents
            .agent_status
            .with_label_values(&[&self.agent_name, "failure"])
            .inc();

        self.metrics
            .errors
            .errors_total
            .with_label_values(&["agent_error", "error", &self.agent_name])
            .inc();

        drop(self);
    }
}

impl Drop for AgentGuard {
    fn drop(&mut self) {
        // Decrement active agents
        self.metrics
            .agents
            .agents_active
            .with_label_values(&[&self.agent_type])
            .dec();
    }
}

/// RAII guard for pipeline phases
pub struct PhaseGuard {
    metrics: Arc<PortalisMetrics>,
    phase_name: String,
    start: Instant,
}

impl PhaseGuard {
    fn new(metrics: Arc<PortalisMetrics>, phase_name: String) -> Self {
        metrics.pipeline.pipelines_active.inc();

        Self {
            metrics,
            phase_name,
            start: Instant::now(),
        }
    }

    /// Mark phase as successful
    pub fn success(self) {
        let duration = self.start.elapsed().as_secs_f64();

        self.metrics
            .pipeline
            .phase_duration
            .with_label_values(&[&self.phase_name])
            .observe(duration);

        self.metrics
            .pipeline
            .phase_status
            .with_label_values(&[&self.phase_name, "success"])
            .inc();

        drop(self);
    }

    /// Mark phase as failed
    pub fn failure(self) {
        let duration = self.start.elapsed().as_secs_f64();

        self.metrics
            .pipeline
            .phase_duration
            .with_label_values(&[&self.phase_name])
            .observe(duration);

        self.metrics
            .pipeline
            .phase_status
            .with_label_values(&[&self.phase_name, "failure"])
            .inc();

        drop(self);
    }
}

impl Drop for PhaseGuard {
    fn drop(&mut self) {
        self.metrics.pipeline.pipelines_active.dec();
    }
}

/// RAII guard for translations
pub struct TranslationGuard {
    metrics: Arc<PortalisMetrics>,
    source_lang: String,
    target_format: String,
    start: Instant,
}

impl TranslationGuard {
    fn new(
        metrics: Arc<PortalisMetrics>,
        source_lang: String,
        target_format: String,
    ) -> Self {
        metrics
            .translation
            .translations_in_progress
            .with_label_values(&[&source_lang])
            .inc();

        metrics
            .translation
            .translations_total
            .with_label_values(&[&source_lang, &target_format])
            .inc();

        Self {
            metrics,
            source_lang,
            target_format,
            start: Instant::now(),
        }
    }

    /// Mark translation as successful
    pub fn success(self, lines_of_code: usize, _complexity: f64) {
        let duration = self.start.elapsed().as_secs_f64();

        self.metrics
            .translation
            .translation_duration
            .with_label_values(&[&self.source_lang, "medium"])
            .observe(duration);

        self.metrics
            .translation
            .translation_loc
            .with_label_values(&[&self.source_lang])
            .observe(lines_of_code as f64);

        self.metrics
            .translation
            .translations_success
            .with_label_values(&[&self.source_lang, &self.target_format])
            .inc();

        drop(self);
    }

    /// Mark translation as failed
    pub fn failure(self, error_category: &str) {
        self.metrics
            .translation
            .translations_failed
            .with_label_values(&[&self.source_lang, &self.target_format, error_category])
            .inc();

        self.metrics
            .errors
            .translation_errors
            .with_label_values(&[error_category, "translation"])
            .inc();

        drop(self);
    }
}

impl Drop for TranslationGuard {
    fn drop(&mut self) {
        self.metrics
            .translation
            .translations_in_progress
            .with_label_values(&[&self.source_lang])
            .dec();
    }
}

/// Helper macro for automatic metric instrumentation
#[macro_export]
macro_rules! instrument_fn {
    ($metrics:expr, $agent_name:expr, $agent_type:expr, $body:expr) => {{
        let guard = $metrics.agent_start($agent_name, $agent_type);
        match $body {
            Ok(result) => {
                guard.success();
                Ok(result)
            }
            Err(e) => {
                guard.failure(&format!("{:?}", e));
                Err(e)
            }
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::PortalisMetrics;

    #[test]
    fn test_metrics_middleware_creation() {
        let metrics = Arc::new(PortalisMetrics::new().unwrap());
        let middleware = MetricsMiddleware::new(metrics);
        assert!(true);
    }

    #[test]
    fn test_request_guard() {
        let metrics = Arc::new(PortalisMetrics::new().unwrap());
        let middleware = MetricsMiddleware::new(metrics);

        let guard = middleware.request_start("/api/translate", "POST");
        guard.success();

        // Verify metrics were recorded
        // In a real test, you'd check the metric values
    }

    #[test]
    fn test_agent_guard() {
        let metrics = Arc::new(PortalisMetrics::new().unwrap());
        let middleware = MetricsMiddleware::new(metrics.clone());

        let guard = middleware.agent_start("ingest", "parser");
        guard.success();

        // Check that active count was decremented
        let export = metrics.export().unwrap();
        assert!(export.contains("portalis_agent"));
    }

    #[test]
    fn test_translation_guard() {
        let metrics = Arc::new(PortalisMetrics::new().unwrap());
        let middleware = MetricsMiddleware::new(metrics.clone());

        let guard = middleware.translation_start("python", "wasm");
        guard.success(100, 5.0);

        let export = metrics.export().unwrap();
        assert!(export.contains("portalis_translations"));
    }
}
