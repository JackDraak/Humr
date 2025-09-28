#[cfg(test)]
mod error_recovery_tests {
    use crate::error_recovery::*;
    use std::collections::HashMap;

    #[test]
    fn test_error_recovery_manager_creation() {
        let manager = ErrorRecoveryManager::new();

        // Manager should be created successfully
        assert!(manager.get_error_history().is_ok());
        assert!(manager.get_circuit_breaker_status().is_ok());
    }

    #[test]
    fn test_audio_error_recovery() {
        let manager = ErrorRecoveryManager::new();

        let error = create_audio_error(
            "Audio device initialization failed".to_string(),
            ErrorSeverity::Medium
        );

        let result = manager.handle_error(error);
        assert!(result.is_ok());

        // Check that error was recorded
        let history = manager.get_error_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].category, ErrorCategory::Audio);
    }

    #[test]
    fn test_network_error_recovery() {
        let manager = ErrorRecoveryManager::new();

        let error = create_network_error(
            "Network connection timeout".to_string(),
            ErrorSeverity::High
        );

        let result = manager.handle_error(error);
        assert!(result.is_ok());

        // Check that error was recorded
        let history = manager.get_error_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].category, ErrorCategory::Network);
    }

    #[test]
    fn test_security_error_recovery() {
        let manager = ErrorRecoveryManager::new();

        let error = create_security_error(
            "Key generation failed".to_string(),
            ErrorSeverity::Critical
        );

        let result = manager.handle_error(error);
        assert!(result.is_ok());

        // Check that error was recorded
        let history = manager.get_error_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].category, ErrorCategory::Security);
    }

    #[test]
    fn test_configuration_error_recovery() {
        let manager = ErrorRecoveryManager::new();

        let error = create_config_error(
            "Configuration file corrupted".to_string(),
            ErrorSeverity::Medium
        );

        let result = manager.handle_error(error);
        assert!(result.is_ok());

        // Check that error was recorded
        let history = manager.get_error_history().unwrap();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].category, ErrorCategory::Configuration);
    }

    #[test]
    fn test_circuit_breaker_functionality() {
        let manager = ErrorRecoveryManager::new();

        // Create multiple errors of the same type to trigger circuit breaker
        for i in 0..6 {
            let error = create_audio_error(
                format!("Audio device failure {}", i),
                ErrorSeverity::High
            );

            let _ = manager.handle_error(error);
        }

        // Check circuit breaker status
        let status = manager.get_circuit_breaker_status().unwrap();
        // Circuit breaker should be triggered after multiple failures
        assert!(!status.is_empty());
    }

    #[test]
    fn test_error_history_management() {
        let manager = ErrorRecoveryManager::new();

        // Add several errors
        for i in 0..5 {
            let error = create_audio_error(
                format!("Test error {}", i),
                ErrorSeverity::Low
            );
            let _ = manager.handle_error(error);
        }

        let history = manager.get_error_history().unwrap();
        assert_eq!(history.len(), 5);

        // Verify error ordering (most recent first or chronological)
        for (i, error) in history.iter().enumerate() {
            assert!(error.message.contains(&format!("Test error {}", i)));
        }
    }

    #[test]
    fn test_error_severity_levels() {
        let error_low = create_audio_error("Low severity".to_string(), ErrorSeverity::Low);
        let error_medium = create_audio_error("Medium severity".to_string(), ErrorSeverity::Medium);
        let error_high = create_audio_error("High severity".to_string(), ErrorSeverity::High);
        let error_critical = create_audio_error("Critical severity".to_string(), ErrorSeverity::Critical);

        assert_eq!(error_low.severity, ErrorSeverity::Low);
        assert_eq!(error_medium.severity, ErrorSeverity::Medium);
        assert_eq!(error_high.severity, ErrorSeverity::High);
        assert_eq!(error_critical.severity, ErrorSeverity::Critical);
    }

    #[test]
    fn test_error_categories() {
        let audio_error = create_audio_error("Audio test".to_string(), ErrorSeverity::Low);
        let network_error = create_network_error("Network test".to_string(), ErrorSeverity::Low);
        let security_error = create_security_error("Security test".to_string(), ErrorSeverity::Low);
        let config_error = create_config_error("Config test".to_string(), ErrorSeverity::Low);

        assert_eq!(audio_error.category, ErrorCategory::Audio);
        assert_eq!(network_error.category, ErrorCategory::Network);
        assert_eq!(security_error.category, ErrorCategory::Security);
        assert_eq!(config_error.category, ErrorCategory::Configuration);
    }

    #[test]
    fn test_audio_recovery_handler() {
        let handler = AudioRecoveryHandler::new();

        let audio_error = create_audio_error("device error".to_string(), ErrorSeverity::Medium);
        assert!(handler.can_handle(&audio_error));

        let network_error = create_network_error("connection error".to_string(), ErrorSeverity::Medium);
        assert!(!handler.can_handle(&network_error));

        // Test recovery actions
        let actions = handler.get_actions();
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.name == "device_reinit"));
    }

    #[test]
    fn test_network_recovery_handler() {
        let handler = NetworkRecoveryHandler::new();

        let network_error = create_network_error("connection timeout".to_string(), ErrorSeverity::High);
        assert!(handler.can_handle(&network_error));

        let audio_error = create_audio_error("device error".to_string(), ErrorSeverity::Medium);
        assert!(!handler.can_handle(&audio_error));

        // Test recovery actions
        let actions = handler.get_actions();
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.name == "reconnect"));
    }

    #[test]
    fn test_security_recovery_handler() {
        let handler = SecurityRecoveryHandler::new();

        let security_error = create_security_error("key rotation failed".to_string(), ErrorSeverity::Critical);
        assert!(handler.can_handle(&security_error));

        let config_error = create_config_error("invalid config".to_string(), ErrorSeverity::Medium);
        assert!(!handler.can_handle(&config_error));

        // Test recovery actions
        let actions = handler.get_actions();
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.name == "regenerate_keys"));
    }

    #[test]
    fn test_configuration_recovery_handler() {
        let handler = ConfigurationRecoveryHandler::new();

        let config_error = create_config_error("corrupted config file".to_string(), ErrorSeverity::Medium);
        assert!(handler.can_handle(&config_error));

        let security_error = create_security_error("key error".to_string(), ErrorSeverity::High);
        assert!(!handler.can_handle(&security_error));

        // Test recovery actions
        let actions = handler.get_actions();
        assert!(!actions.is_empty());
        assert!(actions.iter().any(|a| a.name == "restore_backup"));
    }

    #[test]
    fn test_error_recovery_with_context() {
        let mut error = create_audio_error("Device error".to_string(), ErrorSeverity::Medium);

        // Add context information
        error.context.insert("device_id".to_string(), "audio_device_0".to_string());
        error.context.insert("error_code".to_string(), "E0001".to_string());

        let manager = ErrorRecoveryManager::new();
        let result = manager.handle_error(error);

        assert!(result.is_ok());

        let history = manager.get_error_history().unwrap();
        assert_eq!(history.len(), 1);
        assert!(history[0].context.contains_key("device_id"));
        assert!(history[0].context.contains_key("error_code"));
    }

    #[test]
    fn test_recovery_attempt_counting() {
        let manager = ErrorRecoveryManager::new();

        let mut error = create_audio_error("Persistent device error".to_string(), ErrorSeverity::High);

        // First attempt
        let _ = manager.handle_error(error.clone());

        // Second attempt
        error.recovery_attempts = 1;
        let _ = manager.handle_error(error.clone());

        let history = manager.get_error_history().unwrap();
        // Should have recorded both attempts
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_recovery_action_validation() {
        let action = RecoveryAction {
            name: "test_action".to_string(),
            description: "Test recovery action".to_string(),
            max_attempts: 3,
            backoff_ms: 1000,
            timeout_ms: 5000,
        };

        assert_eq!(action.name, "test_action");
        assert_eq!(action.max_attempts, 3);
        assert_eq!(action.backoff_ms, 1000);
        assert_eq!(action.timeout_ms, 5000);
    }

    #[test]
    fn test_circuit_breaker_states() {
        // Test that circuit breaker states can be compared
        assert_eq!(CircuitBreakerState::Closed, CircuitBreakerState::Closed);
        assert_eq!(CircuitBreakerState::Open, CircuitBreakerState::Open);
        assert_eq!(CircuitBreakerState::HalfOpen, CircuitBreakerState::HalfOpen);

        assert_ne!(CircuitBreakerState::Closed, CircuitBreakerState::Open);
        assert_ne!(CircuitBreakerState::Open, CircuitBreakerState::HalfOpen);
    }

    #[test]
    fn test_error_event_serialization() {
        let error = create_audio_error("Test error".to_string(), ErrorSeverity::Medium);

        // Test that the error can be serialized (important for logging/persistence)
        let serialized = serde_json::to_string(&error);
        assert!(serialized.is_ok());

        let deserialized: Result<ErrorEvent, _> = serde_json::from_str(&serialized.unwrap());
        assert!(deserialized.is_ok());

        let restored_error = deserialized.unwrap();
        assert_eq!(restored_error.message, error.message);
        assert_eq!(restored_error.category, error.category);
        assert_eq!(restored_error.severity, error.severity);
    }
}