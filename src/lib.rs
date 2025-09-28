//! # Humr Voice Communication System
//!
//! A secure, real-time voice communication system with enterprise-grade features.
//!
//! ## Overview
//!
//! Humr provides high-quality voice communication with end-to-end encryption,
//! noise suppression, echo cancellation, and comprehensive monitoring. The system
//! is designed for production use with extensive error recovery and health monitoring.
//!
//! ## Core Features
//!
//! - **Real-time Audio Processing**: Lock-free audio pipeline with configurable parameters
//! - **Security**: X25519/Ed25519/ChaCha20-Poly1305 encryption with forward secrecy
//! - **Audio Quality**: Advanced noise suppression and echo cancellation
//! - **Network**: UDP-based low-latency communication with jitter buffering
//! - **Monitoring**: Comprehensive health checks and performance metrics
//! - **Configuration**: Persistent configuration management with validation
//! - **Error Recovery**: Circuit breaker pattern with automatic recovery
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use humr::{VocalCommunicationApp, realtime_audio::AudioConfiguration};
//! use anyhow::Result;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     // Create application with custom audio configuration
//!     let audio_config = AudioConfiguration {
//!         sample_rate: 48000,
//!         channels: 2,
//!         frame_duration_ms: 20,
//!         buffer_capacity_multiplier: 25,
//!     };
//!
//!     let mut app = VocalCommunicationApp::with_audio_config(audio_config)?;
//!
//!     // Start the application
//!     app.run().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The system follows a modular architecture with clear separation of concerns:
//!
//! - [`app`]: Main application orchestration and high-level API
//! - [`audio`]: Basic audio processing and device management
//! - [`realtime_audio`]: Lock-free real-time audio processing pipeline
//! - [`security`]: Cryptographic protocols and secure session management
//! - [`network`]: UDP networking with encryption and handshake protocols
//! - [`config`]: Configuration management with persistence and validation
//! - [`monitoring`]: Health monitoring and performance metrics collection
//! - [`error_recovery`]: Error handling with circuit breakers and automatic recovery
//!
//! ## Audio Processing Pipeline
//!
//! The audio processing follows this pipeline:
//!
//! 1. **Input Capture**: Platform-specific audio device capture
//! 2. **Noise Suppression**: Advanced time-domain noise reduction
//! 3. **Echo Cancellation**: Adaptive echo cancellation
//! 4. **Encoding**: Opus audio compression
//! 5. **Encryption**: ChaCha20-Poly1305 authenticated encryption
//! 6. **Network**: UDP transmission with jitter buffering
//! 7. **Decryption**: Message authentication and decryption
//! 8. **Decoding**: Opus audio decompression
//! 9. **Output Playback**: Platform-specific audio device playback
//!
//! ## Security Model
//!
//! - **Forward Secrecy**: Ephemeral X25519 key exchange per session
//! - **Authentication**: Ed25519 digital signatures for identity verification
//! - **Encryption**: ChaCha20-Poly1305 AEAD for message confidentiality and integrity
//! - **Replay Protection**: Nonce-based replay attack prevention
//! - **Trust Management**: TOFU (Trust On First Use) with manual verification support

/// Basic audio processing and device management
pub mod audio;

/// Lock-free real-time audio processing with configurable parameters
pub mod realtime_audio;

/// Adaptive jitter buffer for network packet reordering
pub mod jitter_buffer;

/// Opus audio codec integration for high-quality compression
pub mod opus_codec;

/// Advanced noise suppression with speech preservation
pub mod noise_suppression;

/// Adaptive echo cancellation for full-duplex communication
pub mod echo_cancellation;

/// UDP networking with encryption and secure handshake protocols
pub mod network;

/// Interactive command-line user interface
pub mod ui;

/// Main application orchestration and high-level API
pub mod app;

/// Cross-platform audio device abstraction
pub mod platform;

/// Cryptographic protocols and secure session management
pub mod security;

/// Configuration management with persistence and validation
pub mod config;

/// Health monitoring and performance metrics collection
pub mod monitoring;

/// Error handling with circuit breakers and automatic recovery
pub mod error_recovery;

#[cfg(test)]
pub mod tests;

// Re-export main types for convenience
pub use app::VocalCommunicationApp;
pub use config::AppConfig;
pub use realtime_audio::{AudioConfiguration, RealTimeAudioProcessor};
pub use security::{SecurityConfig, SecureSession};
pub use monitoring::{HealthMonitor, MetricsCollector, HealthReport};