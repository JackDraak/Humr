use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use anyhow::{Result, Context};
use log::{info, warn, error};

use crate::audio::AudioProcessor;
use crate::opus_codec::OpusConfig;
use crate::noise_suppression::NoiseSuppressionConfig;
use crate::echo_cancellation::EchoCancellationConfig;
use crate::network::ConnectionConfig;

/// Persistent application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub audio: AudioSettings,
    pub network: NetworkSettings,
    pub security: SecuritySettings,
    pub processing: ProcessingSettings,
    pub ui: UISettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSettings {
    pub input_device: Option<String>,
    pub output_device: Option<String>,
    pub sample_rate: u32,
    pub bit_rate: u32,
    pub input_gain: f32,
    pub output_volume: u8,
    pub buffer_size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSettings {
    pub remote_host: String,
    pub port: u16,
    pub auto_connect: bool,
    pub connection_timeout_ms: u32,
    pub keepalive_interval_ms: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    pub encryption_enabled: bool,
    pub key_rotation_interval_ms: u32,
    pub trusted_peers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingSettings {
    pub noise_suppression: NoiseSuppressionSettings,
    pub echo_cancellation: EchoCancellationSettings,
    pub codec: CodecSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NoiseSuppressionSettings {
    pub enabled: bool,
    pub strength: f32,
    pub adaptive: bool,
    pub noise_floor_db: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoCancellationSettings {
    pub enabled: bool,
    pub filter_length: usize,
    pub max_echo_delay_ms: f32,
    pub suppression_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodecSettings {
    pub bitrate: u32,
    pub complexity: u32,
    pub fec_enabled: bool,
    pub dtx_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UISettings {
    pub theme: String,
    pub auto_save_config: bool,
    pub show_advanced_settings: bool,
    pub minimize_to_tray: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            audio: AudioSettings::default(),
            network: NetworkSettings::default(),
            security: SecuritySettings::default(),
            processing: ProcessingSettings::default(),
            ui: UISettings::default(),
        }
    }
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            input_device: None,
            output_device: None,
            sample_rate: 48000,
            bit_rate: 64000,
            input_gain: 1.0,
            output_volume: 80,
            buffer_size: 1024,
        }
    }
}

impl Default for NetworkSettings {
    fn default() -> Self {
        Self {
            remote_host: "localhost".to_string(),
            port: 8080,
            auto_connect: false,
            connection_timeout_ms: 5000,
            keepalive_interval_ms: 30000,
        }
    }
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            encryption_enabled: true,
            key_rotation_interval_ms: 300000, // 5 minutes
            trusted_peers: Vec::new(),
        }
    }
}

impl Default for ProcessingSettings {
    fn default() -> Self {
        Self {
            noise_suppression: NoiseSuppressionSettings::default(),
            echo_cancellation: EchoCancellationSettings::default(),
            codec: CodecSettings::default(),
        }
    }
}

impl Default for NoiseSuppressionSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            strength: 0.7,
            adaptive: true,
            noise_floor_db: -50.0,
        }
    }
}

impl Default for EchoCancellationSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            filter_length: 512,
            max_echo_delay_ms: 200.0,
            suppression_enabled: true,
        }
    }
}

impl Default for CodecSettings {
    fn default() -> Self {
        Self {
            bitrate: 64000,
            complexity: 5,
            fec_enabled: true,
            dtx_enabled: true,
        }
    }
}

impl Default for UISettings {
    fn default() -> Self {
        Self {
            theme: "dark".to_string(),
            auto_save_config: true,
            show_advanced_settings: false,
            minimize_to_tray: true,
        }
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    config: AppConfig,
}

impl ConfigManager {
    pub fn with_config(config: AppConfig) -> Self {
        Self {
            config_path: PathBuf::from("fallback_config.toml"),
            config,
        }
    }

    pub fn new() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        let config = Self::load_or_create_config(&config_path)?;

        Ok(Self {
            config_path,
            config,
        })
    }

    pub fn get_config(&self) -> &AppConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: AppConfig) -> Result<()> {
        self.config = config;
        self.save_config()
    }

    pub fn save_config(&self) -> Result<()> {
        let config_str = toml::to_string_pretty(&self.config)
            .context("Failed to serialize configuration")?;

        // Ensure parent directory exists
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        fs::write(&self.config_path, config_str)
            .context("Failed to write configuration file")?;

        info!("Configuration saved to: {:?}", self.config_path);
        Ok(())
    }

    fn load_or_create_config(config_path: &PathBuf) -> Result<AppConfig> {
        if config_path.exists() {
            info!("Loading configuration from: {:?}", config_path);
            let config_str = fs::read_to_string(config_path)
                .context("Failed to read configuration file")?;

            match toml::from_str::<AppConfig>(&config_str) {
                Ok(config) => {
                    info!("Configuration loaded successfully");
                    Ok(config)
                }
                Err(e) => {
                    warn!("Failed to parse configuration file: {}. Using defaults.", e);
                    let default_config = AppConfig::default();
                    // Try to save the default config
                    if let Err(save_err) = Self::save_config_to_path(&default_config, config_path) {
                        error!("Failed to save default configuration: {}", save_err);
                    }
                    Ok(default_config)
                }
            }
        } else {
            info!("No configuration file found. Creating default configuration.");
            let default_config = AppConfig::default();

            // Save default configuration
            Self::save_config_to_path(&default_config, config_path)?;

            Ok(default_config)
        }
    }

    fn save_config_to_path(config: &AppConfig, path: &PathBuf) -> Result<()> {
        let config_str = toml::to_string_pretty(config)
            .context("Failed to serialize default configuration")?;

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create config directory")?;
        }

        fs::write(path, config_str)
            .context("Failed to write default configuration file")?;

        info!("Default configuration saved to: {:?}", path);
        Ok(())
    }

    fn get_config_path() -> Result<PathBuf> {
        let config_dir = if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("humr")
        } else {
            // Fallback to home directory
            let home_dir = dirs::home_dir()
                .context("Could not determine home directory")?;
            home_dir.join(".humr")
        };

        Ok(config_dir.join("config.toml"))
    }
}

// Conversion methods to integrate with existing systems
impl AppConfig {
    pub fn to_opus_config(&self) -> OpusConfig {
        OpusConfig {
            bitrate: self.processing.codec.bitrate,
            complexity: self.processing.codec.complexity,
            fec_enabled: self.processing.codec.fec_enabled,
            dtx_enabled: self.processing.codec.dtx_enabled,
            ..OpusConfig::default()
        }
    }

    pub fn to_noise_suppression_config(&self) -> NoiseSuppressionConfig {
        NoiseSuppressionConfig {
            strength: self.processing.noise_suppression.strength,
            noise_floor_db: self.processing.noise_suppression.noise_floor_db,
            adaptive: self.processing.noise_suppression.adaptive,
            ..NoiseSuppressionConfig::default()
        }
    }

    pub fn to_echo_cancellation_config(&self) -> EchoCancellationConfig {
        EchoCancellationConfig {
            filter_length: self.processing.echo_cancellation.filter_length,
            max_echo_delay_ms: self.processing.echo_cancellation.max_echo_delay_ms,
            ..EchoCancellationConfig::default()
        }
    }

    pub fn to_connection_config(&self) -> ConnectionConfig {
        ConnectionConfig {
            remote_host: self.network.remote_host.clone(),
            port: self.network.port,
            use_encryption: self.security.encryption_enabled,
            security_config: None, // Will be set separately
        }
    }

    pub fn apply_to_audio_processor(&self, processor: &mut AudioProcessor) {
        processor.set_bit_rate(self.audio.bit_rate);
        processor.set_sample_rate(self.audio.sample_rate);
        processor.set_input_gain(self.audio.input_gain);
        processor.set_output_volume(self.audio.output_volume);

        if let Some(ref device) = self.audio.input_device {
            processor.set_input_device(device);
        }
        if let Some(ref device) = self.audio.output_device {
            processor.set_output_device(device);
        }

        processor.enable_noise_cancellation(self.processing.noise_suppression.enabled);
        processor.set_echo_cancellation(self.processing.echo_cancellation.enabled);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_creation() {
        let config = AppConfig::default();
        assert_eq!(config.audio.sample_rate, 48000);
        assert_eq!(config.network.port, 8080);
        assert!(config.security.encryption_enabled);
        assert!(config.processing.noise_suppression.enabled);
    }

    #[test]
    fn test_config_serialization() {
        let config = AppConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: AppConfig = toml::from_str(&serialized).unwrap();

        assert_eq!(config.audio.sample_rate, deserialized.audio.sample_rate);
        assert_eq!(config.network.remote_host, deserialized.network.remote_host);
    }

    #[test]
    fn test_config_manager_creation() {
        // This test would normally set up a temporary config directory
        // For now, just test that ConfigManager can be instantiated
        assert!(ConfigManager::new().is_ok());
    }

    #[test]
    fn test_config_conversions() {
        let config = AppConfig::default();

        let opus_config = config.to_opus_config();
        assert_eq!(opus_config.bitrate, config.processing.codec.bitrate);

        let noise_config = config.to_noise_suppression_config();
        assert_eq!(noise_config.strength, config.processing.noise_suppression.strength);

        let echo_config = config.to_echo_cancellation_config();
        assert_eq!(echo_config.filter_length, config.processing.echo_cancellation.filter_length);

        let connection_config = config.to_connection_config();
        assert_eq!(connection_config.remote_host, config.network.remote_host);
    }
}