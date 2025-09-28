use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use anyhow::Result;
use log::{info, warn, error, debug};

/// System health status levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    pub name: String,
    pub status: HealthStatus,
    pub message: String,
    pub last_updated: u64, // Unix timestamp
    pub duration_ms: u64,
}

/// Overall system health report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    pub overall_status: HealthStatus,
    pub checks: Vec<HealthCheck>,
    pub timestamp: u64,
    pub uptime_seconds: u64,
}

/// Performance metrics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    // Audio processing metrics
    pub audio_processing_latency_ms: f64,
    pub audio_dropouts_per_minute: f64,
    pub audio_buffer_utilization: f64,

    // Network metrics
    pub network_latency_ms: f64,
    pub packet_loss_rate: f64,
    pub bandwidth_utilization_mbps: f64,

    // System metrics
    pub cpu_usage_percent: f64,
    pub memory_usage_mb: f64,
    pub disk_usage_percent: f64,

    // Application metrics
    pub active_connections: u32,
    pub messages_per_second: f64,
    pub error_rate_per_minute: f64,

    pub timestamp: u64,
}

/// Health check function type
pub type HealthCheckFn = Box<dyn Fn() -> Result<HealthCheck> + Send + Sync>;

/// Monitoring and health check system
pub struct HealthMonitor {
    checks: Arc<Mutex<HashMap<String, HealthCheckFn>>>,
    last_report: Arc<Mutex<Option<HealthReport>>>,
    metrics: Arc<Mutex<PerformanceMetrics>>,
    start_time: Instant,
    check_interval: Duration,
}

impl HealthMonitor {
    pub fn new() -> Self {
        Self {
            checks: Arc::new(Mutex::new(HashMap::new())),
            last_report: Arc::new(Mutex::new(None)),
            metrics: Arc::new(Mutex::new(PerformanceMetrics::new())),
            start_time: Instant::now(),
            check_interval: Duration::from_secs(30), // Default 30 second intervals
        }
    }

    /// Register a health check function
    pub fn register_check<F>(&self, name: String, check_fn: F)
    where
        F: Fn() -> Result<HealthCheck> + Send + Sync + 'static,
    {
        if let Ok(mut checks) = self.checks.lock() {
            checks.insert(name.clone(), Box::new(check_fn));
            debug!("Registered health check: {}", name);
        }
    }

    /// Run all health checks and generate a report
    pub fn run_health_checks(&self) -> HealthReport {
        let mut check_results = Vec::new();
        let mut overall_status = HealthStatus::Healthy;

        if let Ok(checks) = self.checks.lock() {
            for (name, check_fn) in checks.iter() {
                let start = Instant::now();

                match check_fn() {
                    Ok(mut result) => {
                        result.duration_ms = start.elapsed().as_millis() as u64;
                        result.last_updated = Self::current_timestamp();

                        // Determine overall status (worst case wins)
                        match result.status {
                            HealthStatus::Critical => overall_status = HealthStatus::Critical,
                            HealthStatus::Warning if overall_status == HealthStatus::Healthy => {
                                overall_status = HealthStatus::Warning;
                            }
                            _ => {}
                        }

                        check_results.push(result);
                    }
                    Err(e) => {
                        let duration = start.elapsed().as_millis() as u64;
                        overall_status = HealthStatus::Critical;

                        check_results.push(HealthCheck {
                            name: name.clone(),
                            status: HealthStatus::Critical,
                            message: format!("Health check failed: {}", e),
                            last_updated: Self::current_timestamp(),
                            duration_ms: duration,
                        });

                        error!("Health check '{}' failed: {}", name, e);
                    }
                }
            }
        }

        let report = HealthReport {
            overall_status,
            checks: check_results,
            timestamp: Self::current_timestamp(),
            uptime_seconds: self.start_time.elapsed().as_secs(),
        };

        // Store the latest report
        if let Ok(mut last_report) = self.last_report.lock() {
            *last_report = Some(report.clone());
        }

        info!("Health check completed - Status: {:?}, Checks: {}",
              report.overall_status, report.checks.len());

        report
    }

    /// Get the latest health report
    pub fn get_latest_report(&self) -> Option<HealthReport> {
        if let Ok(report) = self.last_report.lock() {
            report.clone()
        } else {
            None
        }
    }

    /// Update performance metrics
    pub fn update_metrics(&self, metrics: PerformanceMetrics) {
        if let Ok(mut current_metrics) = self.metrics.lock() {
            *current_metrics = metrics;
            debug!("Performance metrics updated");
        }
    }

    /// Get current performance metrics
    pub fn get_metrics(&self) -> PerformanceMetrics {
        if let Ok(metrics) = self.metrics.lock() {
            metrics.clone()
        } else {
            PerformanceMetrics::new()
        }
    }

    /// Start automatic health checking in background
    pub fn start_monitoring(&self) -> Result<()> {
        let checks = self.checks.clone();
        let last_report = self.last_report.clone();
        let interval = self.check_interval;
        let start_time = self.start_time;

        std::thread::spawn(move || {
            info!("Health monitoring started with interval: {:?}", interval);

            loop {
                std::thread::sleep(interval);

                let mut check_results = Vec::new();
                let mut overall_status = HealthStatus::Healthy;

                if let Ok(checks_guard) = checks.lock() {
                    for (name, check_fn) in checks_guard.iter() {
                        let start = Instant::now();

                        match check_fn() {
                            Ok(mut result) => {
                                result.duration_ms = start.elapsed().as_millis() as u64;
                                result.last_updated = Self::current_timestamp();

                                match result.status {
                                    HealthStatus::Critical => overall_status = HealthStatus::Critical,
                                    HealthStatus::Warning if overall_status == HealthStatus::Healthy => {
                                        overall_status = HealthStatus::Warning;
                                    }
                                    _ => {}
                                }

                                check_results.push(result);
                            }
                            Err(e) => {
                                let duration = start.elapsed().as_millis() as u64;
                                overall_status = HealthStatus::Critical;

                                check_results.push(HealthCheck {
                                    name: name.clone(),
                                    status: HealthStatus::Critical,
                                    message: format!("Health check failed: {}", e),
                                    last_updated: Self::current_timestamp(),
                                    duration_ms: duration,
                                });

                                warn!("Background health check '{}' failed: {}", name, e);
                            }
                        }
                    }
                }

                let report = HealthReport {
                    overall_status,
                    checks: check_results,
                    timestamp: Self::current_timestamp(),
                    uptime_seconds: start_time.elapsed().as_secs(),
                };

                if let Ok(mut last_report_guard) = last_report.lock() {
                    *last_report_guard = Some(report.clone());
                }

                // Log critical status changes
                if report.overall_status == HealthStatus::Critical {
                    error!("System health is CRITICAL - {} checks failed",
                           report.checks.iter().filter(|c| c.status == HealthStatus::Critical).count());
                }
            }
        });

        Ok(())
    }

    fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Set health check interval
    pub fn set_check_interval(&mut self, interval: Duration) {
        self.check_interval = interval;
        info!("Health check interval set to: {:?}", interval);
    }
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            audio_processing_latency_ms: 0.0,
            audio_dropouts_per_minute: 0.0,
            audio_buffer_utilization: 0.0,
            network_latency_ms: 0.0,
            packet_loss_rate: 0.0,
            bandwidth_utilization_mbps: 0.0,
            cpu_usage_percent: 0.0,
            memory_usage_mb: 0.0,
            disk_usage_percent: 0.0,
            active_connections: 0,
            messages_per_second: 0.0,
            error_rate_per_minute: 0.0,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Calculate overall system health score (0.0 = critical, 1.0 = perfect)
    pub fn health_score(&self) -> f64 {
        let mut score = 1.0;

        // Penalize high latency (>100ms is bad)
        if self.audio_processing_latency_ms > 100.0 {
            score *= 1.0 - (self.audio_processing_latency_ms - 100.0) / 500.0;
        }

        // Penalize packet loss (>1% is bad)
        if self.packet_loss_rate > 0.01 {
            score *= 1.0 - (self.packet_loss_rate - 0.01) / 0.1;
        }

        // Penalize high CPU usage (>80% is bad)
        if self.cpu_usage_percent > 80.0 {
            score *= 1.0 - (self.cpu_usage_percent - 80.0) / 20.0;
        }

        // Penalize high error rate (>10/min is bad)
        if self.error_rate_per_minute > 10.0 {
            score *= 1.0 - (self.error_rate_per_minute - 10.0) / 50.0;
        }

        score.max(0.0).min(1.0)
    }
}

/// Default health checks for common system components
pub struct DefaultHealthChecks;

impl DefaultHealthChecks {
    /// Audio system health check
    pub fn audio_system() -> HealthCheckFn {
        Box::new(|| {
            // This would typically check audio devices, buffer status, etc.
            // For now, we'll simulate a basic check

            let status = HealthStatus::Healthy;
            let message = "Audio system operational".to_string();

            Ok(HealthCheck {
                name: "audio_system".to_string(),
                status,
                message,
                last_updated: 0, // Will be set by monitor
                duration_ms: 0,  // Will be set by monitor
            })
        })
    }

    /// Network connectivity health check
    pub fn network_connectivity() -> HealthCheckFn {
        Box::new(|| {
            // This would typically ping network endpoints, check socket status, etc.
            let status = HealthStatus::Healthy;
            let message = "Network connectivity normal".to_string();

            Ok(HealthCheck {
                name: "network_connectivity".to_string(),
                status,
                message,
                last_updated: 0,
                duration_ms: 0,
            })
        })
    }

    /// Memory usage health check
    pub fn memory_usage() -> HealthCheckFn {
        Box::new(|| {
            // This would check actual memory usage
            // For now, simulate a check
            let status = HealthStatus::Healthy;
            let message = "Memory usage within acceptable limits".to_string();

            Ok(HealthCheck {
                name: "memory_usage".to_string(),
                status,
                message,
                last_updated: 0,
                duration_ms: 0,
            })
        })
    }

    /// Disk space health check
    pub fn disk_space() -> HealthCheckFn {
        Box::new(|| {
            // This would check available disk space
            let status = HealthStatus::Healthy;
            let message = "Sufficient disk space available".to_string();

            Ok(HealthCheck {
                name: "disk_space".to_string(),
                status,
                message,
                last_updated: 0,
                duration_ms: 0,
            })
        })
    }

    /// Configuration health check
    pub fn configuration() -> HealthCheckFn {
        Box::new(|| {
            // This would validate configuration integrity
            let status = HealthStatus::Healthy;
            let message = "Configuration valid and accessible".to_string();

            Ok(HealthCheck {
                name: "configuration".to_string(),
                status,
                message,
                last_updated: 0,
                duration_ms: 0,
            })
        })
    }
}

/// Metrics collector for gathering system performance data
pub struct MetricsCollector {
    monitor: Arc<HealthMonitor>,
}

impl MetricsCollector {
    pub fn new(monitor: Arc<HealthMonitor>) -> Self {
        Self { monitor }
    }

    /// Collect and update all performance metrics
    pub fn collect_metrics(&self) -> Result<()> {
        let mut metrics = PerformanceMetrics::new();

        // Collect audio metrics (would integrate with real audio processor)
        metrics.audio_processing_latency_ms = self.collect_audio_latency()?;
        metrics.audio_dropouts_per_minute = self.collect_audio_dropouts()?;
        metrics.audio_buffer_utilization = self.collect_buffer_utilization()?;

        // Collect network metrics (would integrate with real network manager)
        metrics.network_latency_ms = self.collect_network_latency()?;
        metrics.packet_loss_rate = self.collect_packet_loss_rate()?;
        metrics.bandwidth_utilization_mbps = self.collect_bandwidth_utilization()?;

        // Collect system metrics (would use system APIs)
        metrics.cpu_usage_percent = self.collect_cpu_usage()?;
        metrics.memory_usage_mb = self.collect_memory_usage()?;
        metrics.disk_usage_percent = self.collect_disk_usage()?;

        // Collect application metrics
        metrics.active_connections = self.collect_active_connections()?;
        metrics.messages_per_second = self.collect_message_rate()?;
        metrics.error_rate_per_minute = self.collect_error_rate()?;

        self.monitor.update_metrics(metrics);
        debug!("Metrics collection completed");

        Ok(())
    }

    // Placeholder metric collection methods - in production these would interface with real systems
    fn collect_audio_latency(&self) -> Result<f64> { Ok(15.0) } // 15ms typical
    fn collect_audio_dropouts(&self) -> Result<f64> { Ok(0.1) } // 0.1 per minute
    fn collect_buffer_utilization(&self) -> Result<f64> { Ok(0.75) } // 75%
    fn collect_network_latency(&self) -> Result<f64> { Ok(50.0) } // 50ms
    fn collect_packet_loss_rate(&self) -> Result<f64> { Ok(0.001) } // 0.1%
    fn collect_bandwidth_utilization(&self) -> Result<f64> { Ok(1.2) } // 1.2 Mbps
    fn collect_cpu_usage(&self) -> Result<f64> { Ok(25.0) } // 25%
    fn collect_memory_usage(&self) -> Result<f64> { Ok(150.0) } // 150MB
    fn collect_disk_usage(&self) -> Result<f64> { Ok(45.0) } // 45%
    fn collect_active_connections(&self) -> Result<u32> { Ok(2) } // 2 connections
    fn collect_message_rate(&self) -> Result<f64> { Ok(50.0) } // 50 msg/sec
    fn collect_error_rate(&self) -> Result<f64> { Ok(0.5) } // 0.5 errors/min
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_monitor_creation() {
        let monitor = HealthMonitor::new();
        assert!(monitor.get_latest_report().is_none());
    }

    #[test]
    fn test_health_check_registration() {
        let monitor = HealthMonitor::new();

        monitor.register_check("test_check".to_string(), || {
            Ok(HealthCheck {
                name: "test_check".to_string(),
                status: HealthStatus::Healthy,
                message: "Test passed".to_string(),
                last_updated: 0,
                duration_ms: 0,
            })
        });

        let report = monitor.run_health_checks();
        assert_eq!(report.checks.len(), 1);
        assert_eq!(report.checks[0].name, "test_check");
        assert_eq!(report.checks[0].status, HealthStatus::Healthy);
        assert_eq!(report.overall_status, HealthStatus::Healthy);
    }

    #[test]
    fn test_health_status_prioritization() {
        let monitor = HealthMonitor::new();

        monitor.register_check("healthy_check".to_string(), || {
            Ok(HealthCheck {
                name: "healthy_check".to_string(),
                status: HealthStatus::Healthy,
                message: "All good".to_string(),
                last_updated: 0,
                duration_ms: 0,
            })
        });

        monitor.register_check("warning_check".to_string(), || {
            Ok(HealthCheck {
                name: "warning_check".to_string(),
                status: HealthStatus::Warning,
                message: "Some issue".to_string(),
                last_updated: 0,
                duration_ms: 0,
            })
        });

        let report = monitor.run_health_checks();
        assert_eq!(report.overall_status, HealthStatus::Warning);
    }

    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics::new();
        assert_eq!(metrics.active_connections, 0);

        let health_score = metrics.health_score();
        assert!(health_score >= 0.0 && health_score <= 1.0);
    }

    #[test]
    fn test_default_health_checks() {
        let audio_check = DefaultHealthChecks::audio_system();
        let result = audio_check().unwrap();
        assert_eq!(result.name, "audio_system");
        assert_eq!(result.status, HealthStatus::Healthy);

        let network_check = DefaultHealthChecks::network_connectivity();
        let result = network_check().unwrap();
        assert_eq!(result.name, "network_connectivity");
        assert_eq!(result.status, HealthStatus::Healthy);
    }

    #[test]
    fn test_metrics_collector() {
        let monitor = Arc::new(HealthMonitor::new());
        let collector = MetricsCollector::new(monitor.clone());

        assert!(collector.collect_metrics().is_ok());

        let metrics = monitor.get_metrics();
        assert!(metrics.audio_processing_latency_ms > 0.0);
        assert!(metrics.cpu_usage_percent >= 0.0);
    }
}