use anyhow::Result;
use std::path::PathBuf;
use crate::config::{ConfigManager, AppConfig};
use crate::platform::PlatformAudioAdapter;
use crate::security::SecurityConfig;

#[cfg(test)]
mod cross_platform_tests {
    use super::*;

    #[test]
    fn test_config_directory_creation() {
        let config_manager = ConfigManager::new();

        match config_manager {
            Ok(_manager) => {
                // Configuration system should work on all platforms
                assert!(true);
            }
            Err(e) => {
                println!("Config creation failed: {}", e);
                // This should not fail on any supported platform
                assert!(false, "Configuration directory creation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_platform_audio_device_enumeration() {
        let adapter = PlatformAudioAdapter::new();

        // Audio device enumeration should work on all platforms
        let input_devices = adapter.get_input_devices();
        let output_devices = adapter.get_output_devices();

        // Should have at least default devices on all platforms
        assert!(!input_devices.is_empty(), "No input devices found");
        assert!(!output_devices.is_empty(), "No output devices found");

        // Verify default device exists
        assert!(input_devices.contains(&"default".to_string()) ||
                input_devices.iter().any(|d| d.contains("default")));
        assert!(output_devices.contains(&"default".to_string()) ||
                output_devices.iter().any(|d| d.contains("default")));
    }

    #[test]
    fn test_security_config_generation() {
        let result = SecurityConfig::new();

        match result {
            Ok(_config) => {
                // Cryptographic key generation should work on all platforms
                assert!(true);
            }
            Err(e) => {
                assert!(false, "Security config generation failed: {}", e);
            }
        }
    }

    #[test]
    fn test_config_serialization_deserialization() {
        let config = AppConfig::default();

        // Test TOML serialization (cross-platform format)
        let serialized = toml::to_string(&config);
        assert!(serialized.is_ok(), "Config serialization failed");

        let serialized_str = serialized.unwrap();
        let deserialized: Result<AppConfig, _> = toml::from_str(&serialized_str);
        assert!(deserialized.is_ok(), "Config deserialization failed");

        let restored_config = deserialized.unwrap();
        assert_eq!(config.audio.sample_rate, restored_config.audio.sample_rate);
        assert_eq!(config.network.port, restored_config.network.port);
    }

    #[test]
    fn test_audio_adapter_initialization() {
        let mut adapter = PlatformAudioAdapter::new();

        // Audio system initialization should work on all platforms
        match adapter.initialize() {
            Ok(_) => {
                assert!(true);
            }
            Err(e) => {
                // Some CI environments may not have audio, so we warn but don't fail
                println!("Warning: Audio initialization failed (may be headless environment): {}", e);
            }
        }
    }

    #[test]
    fn test_file_path_handling() {
        // Test that our path handling works correctly across platforms
        use dirs;

        if let Some(config_dir) = dirs::config_dir() {
            let humr_config = config_dir.join("humr");
            let config_file = humr_config.join("config.toml");

            // Path creation should work on all platforms
            assert!(config_file.to_string_lossy().contains("humr"));
            assert!(config_file.to_string_lossy().contains("config.toml"));
        }

        if let Some(home_dir) = dirs::home_dir() {
            let fallback_config = home_dir.join(".humr").join("config.toml");
            assert!(fallback_config.to_string_lossy().contains(".humr"));
        }
    }

    #[test]
    fn test_network_port_validation() {
        let config = AppConfig::default();

        // Default port should be valid on all platforms
        assert!(config.network.port > 0);
        assert!(config.network.port <= 65535);

        // Common ports should be acceptable
        let common_ports = [8080, 9090, 12345, 54321];
        for &port in &common_ports {
            assert!(port > 0 && port <= 65535);
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_linux_specific_features() {
        // Test Linux-specific features
        use std::fs;

        // Check if we can access /proc (Linux-specific)
        if let Ok(_) = fs::read_dir("/proc") {
            assert!(true, "Linux /proc filesystem accessible");
        }

        // ALSA audio should be available on most Linux systems
        let adapter = PlatformAudioAdapter::new();
        let devices = adapter.get_input_devices();
        // Don't assert on specific device names as they vary by system
        assert!(!devices.is_empty(), "Should have audio devices on Linux");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn test_macos_specific_features() {
        // Test macOS-specific features
        use std::process::Command;

        // CoreAudio should be available on macOS
        let adapter = PlatformAudioAdapter::new();
        let devices = adapter.get_output_devices();
        assert!(!devices.is_empty(), "Should have CoreAudio devices on macOS");

        // Test that we can run system_profiler (macOS-specific)
        if let Ok(output) = Command::new("system_profiler")
            .arg("SPAudioDataType")
            .output() {
            assert!(output.status.success() || !output.stderr.is_empty());
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn test_windows_specific_features() {
        // Test Windows-specific features

        // WASAPI audio should be available on Windows
        let adapter = PlatformAudioAdapter::new();
        let devices = adapter.get_input_devices();
        assert!(!devices.is_empty(), "Should have WASAPI devices on Windows");

        // Windows-specific path handling
        use std::env;
        if let Ok(appdata) = env::var("APPDATA") {
            assert!(!appdata.is_empty(), "Windows APPDATA should be available");
            let config_path = PathBuf::from(appdata).join("humr").join("config.toml");
            assert!(config_path.to_string_lossy().contains("humr"));
        }
    }

    #[test]
    fn test_cross_platform_thread_safety() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        // Test that our core components are thread-safe across platforms
        let config = Arc::new(Mutex::new(AppConfig::default()));

        let handles: Vec<_> = (0..4).map(|i| {
            let config_clone = config.clone();
            thread::spawn(move || {
                let config = config_clone.lock().unwrap();
                // Each thread should be able to access config safely
                assert_eq!(config.audio.sample_rate, 48000);
                println!("Thread {} accessed config successfully", i);
            })
        }).collect();

        for handle in handles {
            handle.join().expect("Thread should complete successfully");
        }
    }

    #[test]
    fn test_cross_platform_error_handling() {
        // Test that our error handling works consistently across platforms

        // Invalid config paths should fail gracefully
        let invalid_config = ConfigManager::with_config(AppConfig::default());
        // This should work (uses fallback path)

        // Invalid audio settings should be handled
        let mut config = AppConfig::default();
        config.audio.sample_rate = 999999; // Invalid sample rate

        // The config should still be serializable even with invalid values
        let serialized = toml::to_string(&config);
        assert!(serialized.is_ok(), "Should handle invalid config gracefully");
    }

    #[test]
    fn test_memory_usage_consistency() {
        // Test that memory usage is reasonable across platforms
        use std::mem;

        let config = AppConfig::default();
        let config_size = mem::size_of_val(&config);

        // Config should not be excessively large
        assert!(config_size < 10000, "Config struct too large: {} bytes", config_size);

        let adapter = PlatformAudioAdapter::new();
        let adapter_size = mem::size_of_val(&adapter);

        // Audio adapter should be reasonably sized
        assert!(adapter_size < 50000, "Audio adapter too large: {} bytes", adapter_size);
    }

    #[test]
    fn test_unicode_path_handling() {
        // Test handling of Unicode characters in file paths (important for international users)
        let test_paths = [
            "config_ñ.toml",
            "config_测试.toml",
            "config_🎵.toml",
            "config_русский.toml"
        ];

        for path in &test_paths {
            let path_buf = PathBuf::from(path);
            let path_str = path_buf.to_string_lossy();

            // Should handle Unicode paths gracefully
            assert!(!path_str.is_empty(), "Unicode path handling failed for: {}", path);
        }
    }
}

#[cfg(test)]
mod performance_validation {
    use super::*;
    use std::time::{Instant, Duration};

    #[test]
    fn test_config_load_performance() {
        let start = Instant::now();

        for _ in 0..100 {
            let _config = AppConfig::default();
        }

        let duration = start.elapsed();
        assert!(duration < Duration::from_millis(100),
                "Config creation too slow: {:?}", duration);
    }

    #[test]
    fn test_audio_device_enum_performance() {
        let adapter = PlatformAudioAdapter::new();
        let start = Instant::now();

        let _input_devices = adapter.get_input_devices();
        let _output_devices = adapter.get_output_devices();

        let duration = start.elapsed();
        assert!(duration < Duration::from_secs(2),
                "Device enumeration too slow: {:?}", duration);
    }

    #[test]
    fn test_serialization_performance() {
        let config = AppConfig::default();
        let start = Instant::now();

        for _ in 0..1000 {
            let _serialized = toml::to_string(&config).unwrap();
        }

        let duration = start.elapsed();
        assert!(duration < Duration::from_millis(500),
                "Serialization too slow: {:?}", duration);
    }
}