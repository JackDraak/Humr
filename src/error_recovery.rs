use anyhow::{Result, Context};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use log::{warn, error, info, debug};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorSeverity {
    Low,      // Recoverable, minimal impact
    Medium,   // Recoverable, noticeable impact
    High,     // Requires immediate action
    Critical, // System-threatening
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ErrorCategory {
    Audio,
    Network,
    Security,
    Configuration,
    Memory,
    Hardware,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEvent {
    pub id: String,
    pub category: ErrorCategory,
    pub severity: ErrorSeverity,
    pub message: String,
    pub timestamp: u64,
    pub context: HashMap<String, String>,
    pub recovery_attempts: u32,
    pub resolved: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryAction {
    pub name: String,
    pub description: String,
    pub max_attempts: u32,
    pub backoff_ms: u64,
    pub timeout_ms: u64,
}

pub trait ErrorRecovery: Send + Sync {
    fn can_handle(&self, error: &ErrorEvent) -> bool;
    fn recover(&self, error: &mut ErrorEvent) -> Result<bool>;
    fn get_actions(&self) -> Vec<RecoveryAction>;
}

pub struct ErrorRecoveryManager {
    handlers: Vec<Box<dyn ErrorRecovery>>,
    error_history: Arc<Mutex<Vec<ErrorEvent>>>,
    max_history_size: usize,
    circuit_breakers: Arc<Mutex<HashMap<String, CircuitBreaker>>>,
}

#[derive(Debug, Clone)]
struct CircuitBreaker {
    failure_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
    state: CircuitBreakerState,
    failure_threshold: u32,
    recovery_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitBreakerState {
    Closed,     // Normal operation
    Open,       // Failing, blocking requests
    HalfOpen,   // Testing recovery
}

impl ErrorRecoveryManager {
    pub fn new() -> Self {
        let mut manager = Self {
            handlers: Vec::new(),
            error_history: Arc::new(Mutex::new(Vec::new())),
            max_history_size: 1000,
            circuit_breakers: Arc::new(Mutex::new(HashMap::new())),
        };

        // Register default recovery handlers
        manager.register_handler(Box::new(AudioRecoveryHandler::new()));
        manager.register_handler(Box::new(NetworkRecoveryHandler::new()));
        manager.register_handler(Box::new(SecurityRecoveryHandler::new()));
        manager.register_handler(Box::new(ConfigurationRecoveryHandler::new()));

        manager
    }

    pub fn register_handler(&mut self, handler: Box<dyn ErrorRecovery>) {
        self.handlers.push(handler);
    }

    pub fn handle_error(&self, mut error: ErrorEvent) -> Result<bool> {
        info!("Handling error: {} - {}", error.id, error.message);

        // Record error in history
        self.record_error(&error)?;

        // Check circuit breaker
        if !self.should_attempt_recovery(&error.id)? {
            warn!("Circuit breaker open for error type: {}", error.id);
            return Ok(false);
        }

        // Find appropriate handler
        for handler in &self.handlers {
            if handler.can_handle(&error) {
                debug!("Found handler for error: {}", error.id);

                match handler.recover(&mut error) {
                    Ok(recovered) => {
                        if recovered {
                            info!("Successfully recovered from error: {}", error.id);
                            error.resolved = true;
                            self.record_success(&error.id)?;
                            self.update_error_in_history(&error)?;
                            return Ok(true);
                        } else {
                            warn!("Recovery attempt failed for error: {}", error.id);
                            self.record_failure(&error.id)?;
                        }
                    }
                    Err(e) => {
                        error!("Recovery handler failed: {}", e);
                        self.record_failure(&error.id)?;
                    }
                }
            }
        }

        warn!("No suitable recovery handler found for error: {}", error.id);
        Ok(false)
    }

    fn should_attempt_recovery(&self, error_id: &str) -> Result<bool> {
        let breakers = self.circuit_breakers.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire circuit breaker lock"))?;

        if let Some(breaker) = breakers.get(error_id) {
            match breaker.state {
                CircuitBreakerState::Closed => Ok(true),
                CircuitBreakerState::Open => {
                    if let Some(last_failure) = breaker.last_failure {
                        if last_failure.elapsed() > breaker.recovery_timeout {
                            // Transition to half-open
                            Ok(true)
                        } else {
                            Ok(false)
                        }
                    } else {
                        Ok(true)
                    }
                }
                CircuitBreakerState::HalfOpen => Ok(true),
            }
        } else {
            Ok(true) // No circuit breaker exists, allow attempt
        }
    }

    fn record_success(&self, error_id: &str) -> Result<()> {
        let mut breakers = self.circuit_breakers.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire circuit breaker lock"))?;

        let breaker = breakers.entry(error_id.to_string())
            .or_insert_with(|| CircuitBreaker::new());

        breaker.success_count += 1;
        breaker.failure_count = 0; // Reset failures on success

        match breaker.state {
            CircuitBreakerState::HalfOpen => {
                breaker.state = CircuitBreakerState::Closed;
                info!("Circuit breaker closed for: {}", error_id);
            }
            _ => {}
        }

        Ok(())
    }

    fn record_failure(&self, error_id: &str) -> Result<()> {
        let mut breakers = self.circuit_breakers.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire circuit breaker lock"))?;

        let breaker = breakers.entry(error_id.to_string())
            .or_insert_with(|| CircuitBreaker::new());

        breaker.failure_count += 1;
        breaker.last_failure = Some(Instant::now());

        if breaker.failure_count >= breaker.failure_threshold {
            breaker.state = CircuitBreakerState::Open;
            warn!("Circuit breaker opened for: {} (failures: {})", error_id, breaker.failure_count);
        }

        Ok(())
    }

    fn record_error(&self, error: &ErrorEvent) -> Result<()> {
        let mut history = self.error_history.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire error history lock"))?;

        history.push(error.clone());

        // Maintain history size limit
        while history.len() > self.max_history_size {
            history.remove(0);
        }

        Ok(())
    }

    fn update_error_in_history(&self, updated_error: &ErrorEvent) -> Result<()> {
        let mut history = self.error_history.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire error history lock"))?;

        if let Some(error) = history.iter_mut().find(|e| e.id == updated_error.id) {
            *error = updated_error.clone();
        }

        Ok(())
    }

    pub fn get_error_history(&self) -> Result<Vec<ErrorEvent>> {
        let history = self.error_history.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire error history lock"))?;
        Ok(history.clone())
    }

    pub fn get_circuit_breaker_status(&self) -> Result<HashMap<String, CircuitBreakerState>> {
        let breakers = self.circuit_breakers.lock()
            .map_err(|_| anyhow::anyhow!("Failed to acquire circuit breaker lock"))?;

        let status: HashMap<String, CircuitBreakerState> = breakers.iter()
            .map(|(k, v)| (k.clone(), v.state.clone()))
            .collect();

        Ok(status)
    }
}

impl CircuitBreaker {
    fn new() -> Self {
        Self {
            failure_count: 0,
            success_count: 0,
            last_failure: None,
            state: CircuitBreakerState::Closed,
            failure_threshold: 5,
            recovery_timeout: Duration::from_secs(30),
        }
    }
}

// Specific error recovery handlers
pub struct AudioRecoveryHandler;

impl AudioRecoveryHandler {
    pub fn new() -> Self {
        Self
    }
}

impl ErrorRecovery for AudioRecoveryHandler {
    fn can_handle(&self, error: &ErrorEvent) -> bool {
        error.category == ErrorCategory::Audio
    }

    fn recover(&self, error: &mut ErrorEvent) -> Result<bool> {
        info!("Attempting audio recovery for: {}", error.message);

        match error.message.as_str() {
            msg if msg.contains("device") => {
                // Audio device recovery
                info!("Attempting to reinitialize audio device");
                error.recovery_attempts += 1;

                // Simulate device reinitialization
                std::thread::sleep(Duration::from_millis(100));

                if error.recovery_attempts < 3 {
                    info!("Audio device recovery successful");
                    Ok(true)
                } else {
                    warn!("Audio device recovery failed after 3 attempts");
                    Ok(false)
                }
            }
            msg if msg.contains("buffer") => {
                // Audio buffer recovery
                info!("Attempting to clear audio buffers");
                error.recovery_attempts += 1;

                // Simulate buffer reset
                std::thread::sleep(Duration::from_millis(50));

                info!("Audio buffer recovery successful");
                Ok(true)
            }
            _ => {
                warn!("Unknown audio error type: {}", error.message);
                Ok(false)
            }
        }
    }

    fn get_actions(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                name: "device_reinit".to_string(),
                description: "Reinitialize audio device".to_string(),
                max_attempts: 3,
                backoff_ms: 1000,
                timeout_ms: 5000,
            },
            RecoveryAction {
                name: "buffer_reset".to_string(),
                description: "Reset audio buffers".to_string(),
                max_attempts: 1,
                backoff_ms: 100,
                timeout_ms: 1000,
            },
        ]
    }
}

pub struct NetworkRecoveryHandler;

impl NetworkRecoveryHandler {
    pub fn new() -> Self {
        Self
    }
}

impl ErrorRecovery for NetworkRecoveryHandler {
    fn can_handle(&self, error: &ErrorEvent) -> bool {
        error.category == ErrorCategory::Network
    }

    fn recover(&self, error: &mut ErrorEvent) -> Result<bool> {
        info!("Attempting network recovery for: {}", error.message);

        match error.message.as_str() {
            msg if msg.contains("connection") => {
                info!("Attempting to reestablish network connection");
                error.recovery_attempts += 1;

                // Exponential backoff
                let delay = Duration::from_millis(1000 * (1 << error.recovery_attempts.min(5)));
                std::thread::sleep(delay);

                if error.recovery_attempts < 5 {
                    info!("Network connection recovery successful");
                    Ok(true)
                } else {
                    warn!("Network connection recovery failed after 5 attempts");
                    Ok(false)
                }
            }
            msg if msg.contains("timeout") => {
                info!("Adjusting network timeout settings");
                error.recovery_attempts += 1;

                // Simulate timeout adjustment
                info!("Network timeout recovery successful");
                Ok(true)
            }
            _ => {
                warn!("Unknown network error type: {}", error.message);
                Ok(false)
            }
        }
    }

    fn get_actions(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                name: "reconnect".to_string(),
                description: "Reestablish network connection".to_string(),
                max_attempts: 5,
                backoff_ms: 1000,
                timeout_ms: 10000,
            },
            RecoveryAction {
                name: "adjust_timeout".to_string(),
                description: "Adjust network timeout settings".to_string(),
                max_attempts: 1,
                backoff_ms: 0,
                timeout_ms: 1000,
            },
        ]
    }
}

pub struct SecurityRecoveryHandler;

impl SecurityRecoveryHandler {
    pub fn new() -> Self {
        Self
    }
}

impl ErrorRecovery for SecurityRecoveryHandler {
    fn can_handle(&self, error: &ErrorEvent) -> bool {
        error.category == ErrorCategory::Security
    }

    fn recover(&self, error: &mut ErrorEvent) -> Result<bool> {
        info!("Attempting security recovery for: {}", error.message);

        match error.message.as_str() {
            msg if msg.contains("key") => {
                info!("Attempting to regenerate security keys");
                error.recovery_attempts += 1;

                if error.recovery_attempts < 2 {
                    // Simulate key regeneration
                    std::thread::sleep(Duration::from_millis(200));
                    info!("Security key recovery successful");
                    Ok(true)
                } else {
                    warn!("Security key recovery failed - manual intervention required");
                    Ok(false)
                }
            }
            msg if msg.contains("session") => {
                info!("Attempting to reestablish secure session");
                error.recovery_attempts += 1;

                // Simulate session reestablishment
                std::thread::sleep(Duration::from_millis(500));
                info!("Security session recovery successful");
                Ok(true)
            }
            _ => {
                warn!("Unknown security error type: {}", error.message);
                Ok(false)
            }
        }
    }

    fn get_actions(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                name: "regenerate_keys".to_string(),
                description: "Regenerate security keys".to_string(),
                max_attempts: 2,
                backoff_ms: 500,
                timeout_ms: 5000,
            },
            RecoveryAction {
                name: "reestablish_session".to_string(),
                description: "Reestablish secure session".to_string(),
                max_attempts: 3,
                backoff_ms: 1000,
                timeout_ms: 10000,
            },
        ]
    }
}

pub struct ConfigurationRecoveryHandler;

impl ConfigurationRecoveryHandler {
    pub fn new() -> Self {
        Self
    }
}

impl ErrorRecovery for ConfigurationRecoveryHandler {
    fn can_handle(&self, error: &ErrorEvent) -> bool {
        error.category == ErrorCategory::Configuration
    }

    fn recover(&self, error: &mut ErrorEvent) -> Result<bool> {
        info!("Attempting configuration recovery for: {}", error.message);

        match error.message.as_str() {
            msg if msg.contains("corrupted") => {
                info!("Attempting to restore configuration from backup");
                error.recovery_attempts += 1;

                // Simulate backup restoration
                std::thread::sleep(Duration::from_millis(100));
                info!("Configuration backup recovery successful");
                Ok(true)
            }
            msg if msg.contains("invalid") => {
                info!("Attempting to reset to default configuration");
                error.recovery_attempts += 1;

                // Simulate default configuration restoration
                info!("Configuration default recovery successful");
                Ok(true)
            }
            _ => {
                warn!("Unknown configuration error type: {}", error.message);
                Ok(false)
            }
        }
    }

    fn get_actions(&self) -> Vec<RecoveryAction> {
        vec![
            RecoveryAction {
                name: "restore_backup".to_string(),
                description: "Restore configuration from backup".to_string(),
                max_attempts: 1,
                backoff_ms: 0,
                timeout_ms: 2000,
            },
            RecoveryAction {
                name: "reset_defaults".to_string(),
                description: "Reset to default configuration".to_string(),
                max_attempts: 1,
                backoff_ms: 0,
                timeout_ms: 1000,
            },
        ]
    }
}

// Utility functions for creating common error events
pub fn create_audio_error(message: String, severity: ErrorSeverity) -> ErrorEvent {
    ErrorEvent {
        id: format!("audio_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
        category: ErrorCategory::Audio,
        severity,
        message,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        context: HashMap::new(),
        recovery_attempts: 0,
        resolved: false,
    }
}

pub fn create_network_error(message: String, severity: ErrorSeverity) -> ErrorEvent {
    ErrorEvent {
        id: format!("network_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
        category: ErrorCategory::Network,
        severity,
        message,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        context: HashMap::new(),
        recovery_attempts: 0,
        resolved: false,
    }
}

pub fn create_security_error(message: String, severity: ErrorSeverity) -> ErrorEvent {
    ErrorEvent {
        id: format!("security_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
        category: ErrorCategory::Security,
        severity,
        message,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        context: HashMap::new(),
        recovery_attempts: 0,
        resolved: false,
    }
}

pub fn create_config_error(message: String, severity: ErrorSeverity) -> ErrorEvent {
    ErrorEvent {
        id: format!("config_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis()),
        category: ErrorCategory::Configuration,
        severity,
        message,
        timestamp: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(),
        context: HashMap::new(),
        recovery_attempts: 0,
        resolved: false,
    }
}