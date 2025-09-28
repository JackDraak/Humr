# Humr Developer Guide

## Architecture Overview

Humr is built with a modular, clean architecture following Uncle Bob's principles. The system is designed for real-time performance with lock-free data structures and minimal allocations.

### Core Modules

```
src/
├── lib.rs              # Public API and module exports
├── app.rs              # Application lifecycle and coordination
├── audio.rs            # Platform audio interface (CPAL integration)
├── realtime_audio.rs   # Lock-free real-time audio pipeline
├── opus_codec.rs       # Opus compression/decompression
├── noise_suppression.rs # Frequency-domain noise reduction
├── echo_cancellation.rs # Adaptive echo cancellation
├── security.rs         # Cryptographic operations
├── networking.rs       # UDP communication protocol
├── monitoring.rs       # Performance metrics and health checks
├── error_recovery.rs   # Graceful error handling
├── platform.rs         # Platform-specific optimizations
└── ui.rs              # User interface and controls
```

## Public API Reference

### Audio Processing

#### `AudioProcessor`

The main audio processing pipeline coordinator.

```rust
use humr::{AudioProcessor, AudioConfig};

// Create processor with default configuration
let processor = AudioProcessor::new(AudioConfig::default())?;

// Start real-time processing
processor.start()?;

// Process audio frame (for custom integration)
let output = processor.process_frame(&input_samples)?;
```

#### Configuration

```rust
use humr::AudioConfig;

let config = AudioConfig {
    sample_rate: 48000,
    buffer_size: 1024,
    channels: 1,
    noise_suppression: true,
    echo_cancellation: true,
};
```

### Security Module

#### `SecureSession`

Handles encrypted communication sessions.

```rust
use humr::{SecureSession, SessionConfig};

// Create new session with forward secrecy
let session = SecureSession::new(SessionConfig {
    key_rotation_interval: Duration::from_secs(3600),
    forward_secrecy: true,
})?;

// Encrypt audio data
let encrypted = session.encrypt(&audio_data)?;

// Decrypt received data
let decrypted = session.decrypt(&encrypted_data)?;
```

### Real-time Audio Pipeline

#### `RealtimeAudioPipeline`

Lock-free audio processing chain.

```rust
use humr::RealtimeAudioPipeline;

let pipeline = RealtimeAudioPipeline::new(config)?;

// Add processing stages
pipeline.add_stage(Box::new(NoiseSuppressionStage::new()?));
pipeline.add_stage(Box::new(EchoCancellationStage::new()?));
pipeline.add_stage(Box::new(OpusEncoderStage::new()?));

// Process audio (non-blocking, real-time safe)
let result = pipeline.process(&input_buffer);
```

## Real-time Programming Guidelines

### Lock-Free Requirements

All audio processing code must be lock-free to avoid priority inversion:

```rust
// ✅ Good: Lock-free ring buffer
use ringbuf::{Consumer, Producer, RingBuffer};

let rb = RingBuffer::<f32>::new(1024);
let (mut prod, mut cons) = rb.split();

// ✅ Good: Atomic operations
use std::sync::atomic::{AtomicBool, Ordering};

let enabled = AtomicBool::new(true);
if enabled.load(Ordering::Relaxed) {
    // Process audio
}

// ❌ Bad: Mutex in audio thread
use std::sync::Mutex;
let data = Mutex::new(vec![]); // Will cause audio dropouts!
```

### Memory Management

Real-time audio processing requires careful memory management:

```rust
// ✅ Good: Pre-allocated buffers
struct AudioProcessor {
    input_buffer: Vec<f32>,
    output_buffer: Vec<f32>,
    temp_buffer: Vec<f32>,
}

impl AudioProcessor {
    fn new(buffer_size: usize) -> Self {
        Self {
            input_buffer: vec![0.0; buffer_size],
            output_buffer: vec![0.0; buffer_size],
            temp_buffer: vec![0.0; buffer_size],
        }
    }

    fn process(&mut self, input: &[f32]) -> &[f32] {
        // Reuse pre-allocated buffers
        self.input_buffer.copy_from_slice(input);
        // Process...
        &self.output_buffer
    }
}

// ❌ Bad: Allocation in audio thread
fn process_audio(input: &[f32]) -> Vec<f32> {
    let mut output = Vec::new(); // Allocation!
    // Process...
    output
}
```

## Testing Framework

### Test Categories

1. **Unit Tests**: Individual module functionality
2. **Integration Tests**: End-to-end audio pipeline
3. **Performance Tests**: Real-time constraints verification
4. **Security Tests**: Cryptographic correctness

### Writing Audio Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_noise_suppression_effectiveness() {
        let mut processor = NoiseSuppressionProcessor::new(48000)?;

        // Generate test signal: sine wave + noise
        let signal = generate_sine_wave(1000.0, 1.0, 48000, 1024);
        let noise = generate_white_noise(0.5, 1024);
        let noisy_signal: Vec<f32> = signal.iter()
            .zip(noise.iter())
            .map(|(s, n)| s + n)
            .collect();

        // Process audio
        let processed = processor.process(&noisy_signal)?;

        // Verify noise reduction
        let signal_power = calculate_signal_power(&processed, 1000.0);
        let noise_power = calculate_noise_power(&processed);
        let snr = signal_power / noise_power;

        assert!(snr > 10.0, "SNR should be > 10dB after processing");
    }

    #[test]
    fn test_real_time_performance() {
        let mut processor = AudioProcessor::new(AudioConfig::default())?;
        let input = vec![0.0f32; 1024];

        // Measure processing time
        let start = Instant::now();
        let _output = processor.process(&input)?;
        let duration = start.elapsed();

        // Must complete within real-time deadline
        let deadline = Duration::from_micros(21333); // ~1024 samples at 48kHz
        assert!(duration < deadline,
               "Processing took {:?}, exceeds real-time deadline of {:?}",
               duration, deadline);
    }
}
```

### Performance Testing

```rust
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_audio_processing(c: &mut Criterion) {
    let mut processor = AudioProcessor::new(AudioConfig::default()).unwrap();
    let input = vec![0.0f32; 1024];

    c.bench_function("audio_processing", |b| {
        b.iter(|| {
            processor.process(&input).unwrap()
        })
    });
}

criterion_group!(benches, bench_audio_processing);
criterion_main!(benches);
```

## Security Implementation

### Cryptographic Operations

All cryptographic operations use the `ring` library for security:

```rust
use ring::{aead, rand};
use x25519_dalek::{EphemeralSecret, PublicKey};

pub struct CryptoManager {
    sealing_key: aead::SealingKey<OneNonceSequence>,
    opening_key: aead::OpeningKey<OneNonceSequence>,
}

impl CryptoManager {
    pub fn new() -> Result<Self, CryptoError> {
        let rng = rand::SystemRandom::new();
        let key_material = aead::UnboundKey::new(&aead::CHACHA20_POLY1305, &key_bytes)?;

        Ok(Self {
            sealing_key: aead::SealingKey::new(key_material, OneNonceSequence::new()),
            opening_key: aead::OpeningKey::new(key_material, OneNonceSequence::new()),
        })
    }

    pub fn encrypt(&mut self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        let mut in_out = data.to_vec();
        let tag = self.sealing_key.seal_in_place_separate_tag(
            aead::Aad::empty(),
            &mut in_out
        )?;

        in_out.extend_from_slice(tag.as_ref());
        Ok(in_out)
    }
}
```

### Key Management

Forward secrecy implementation:

```rust
pub struct ForwardSecretSession {
    current_keys: KeyPair,
    next_keys: KeyPair,
    rotation_interval: Duration,
    last_rotation: Instant,
}

impl ForwardSecretSession {
    pub fn rotate_keys(&mut self) -> Result<(), SecurityError> {
        // Move next keys to current
        self.current_keys = self.next_keys.clone();

        // Generate new next keys
        self.next_keys = KeyPair::generate()?;

        // Clear old key material
        self.current_keys.clear_old_material();

        self.last_rotation = Instant::now();
        Ok(())
    }
}
```

## Platform-Specific Implementation

### Audio Backend Integration

```rust
#[cfg(target_os = "linux")]
mod linux {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    pub fn create_audio_stream() -> Result<cpal::Stream, AudioError> {
        let host = cpal::default_host();
        let device = host.default_input_device()
            .ok_or(AudioError::NoInputDevice)?;

        let config = device.default_input_config()?;
        let stream = device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                // Real-time audio processing
                process_audio_realtime(data);
            },
            move |err| {
                eprintln!("Audio stream error: {}", err);
            },
        )?;

        Ok(stream)
    }
}
```

### Thread Priority Configuration

```rust
#[cfg(unix)]
pub fn set_audio_thread_priority() -> Result<(), PlatformError> {
    use libc::{sched_param, sched_setscheduler, SCHED_FIFO};

    let param = sched_param {
        sched_priority: 80, // High priority for audio
    };

    unsafe {
        if sched_setscheduler(0, SCHED_FIFO, &param) != 0 {
            return Err(PlatformError::ThreadPriorityFailed);
        }
    }

    Ok(())
}
```

## Error Handling

### Error Types

```rust
#[derive(Debug, thiserror::Error)]
pub enum HumrError {
    #[error("Audio device error: {0}")]
    AudioDevice(#[from] cpal::DevicesError),

    #[error("Cryptographic error: {0}")]
    Crypto(#[from] ring::error::Unspecified),

    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("Real-time constraint violation: processing took {actual:?}, deadline was {deadline:?}")]
    RealtimeViolation {
        actual: Duration,
        deadline: Duration,
    },
}
```

### Graceful Degradation

```rust
pub struct ErrorRecoveryManager {
    audio_fallback: bool,
    security_fallback: bool,
    network_retry_count: u32,
}

impl ErrorRecoveryManager {
    pub fn handle_audio_error(&mut self, error: AudioError) -> RecoveryAction {
        match error {
            AudioError::DeviceDisconnected => {
                if !self.audio_fallback {
                    self.audio_fallback = true;
                    RecoveryAction::SwitchToFallbackDevice
                } else {
                    RecoveryAction::DisableAudio
                }
            }
            AudioError::BufferOverrun => RecoveryAction::IncreaseBufferSize,
            AudioError::LatencyTooHigh => RecoveryAction::ReduceProcessingComplexity,
        }
    }
}
```

## Contributing Guidelines

### Code Style

- Use `cargo fmt` for formatting
- Follow Rust naming conventions
- Add documentation for public APIs
- Include examples in doc comments

### Performance Requirements

- Audio processing functions must complete within real-time deadlines
- Memory allocations forbidden in audio thread
- Lock-free algorithms required for real-time code
- CPU usage should not exceed 5% on modern hardware

### Testing Requirements

- Unit tests for all public functions
- Integration tests for end-to-end functionality
- Performance benchmarks for audio processing
- Security tests for cryptographic operations

### Pull Request Process

1. Create feature branch from `main`
2. Implement changes with comprehensive tests
3. Run full test suite: `cargo test`
4. Run benchmarks: `cargo bench`
5. Update documentation if needed
6. Submit PR with detailed description

### Development Setup

```bash
# Clone repository
git clone https://github.com/your-username/humr.git
cd humr

# Install dependencies
cargo build

# Run tests
cargo test

# Run benchmarks
cargo bench

# Check formatting
cargo fmt --check

# Run linter
cargo clippy -- -D warnings
```

## Debugging and Profiling

### Audio Debugging

```bash
# Enable audio debug logging
RUST_LOG=humr::audio=debug cargo run

# Profile audio performance
cargo run --release --features profiling
```

### Security Auditing

```bash
# Run security audit
cargo audit

# Check for unsafe code
cargo geiger
```

### Memory Profiling

```bash
# Use valgrind for memory analysis
valgrind --tool=massif target/release/humr

# Or use cargo-profiler
cargo profiler perf
```