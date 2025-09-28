# Humr Voice Communication System

A high-performance, secure voice communication system built in Rust with real-time audio processing and end-to-end encryption.

## Features

- **Real-time Voice Communication**: Low-latency audio streaming with advanced processing
- **End-to-End Encryption**: Forward secrecy with X25519 key exchange and ChaCha20-Poly1305 AEAD
- **Advanced Audio Processing**: Noise suppression, echo cancellation, and Opus compression
- **Cross-Platform**: Works on Windows, macOS, and Linux
- **Lock-Free Architecture**: High-performance real-time audio pipeline

## Quick Start

### Prerequisites

- Rust 1.70+ (2024 edition)
- Audio devices (microphone and speakers/headphones)

### Installation

```bash
git clone https://github.com/your-username/humr.git
cd humr
cargo build --release
```

### Running

```bash
# Start the voice communication system
cargo run

# Run with debug logging
RUST_LOG=debug cargo run
```

## Audio Processing Pipeline

Humr implements a sophisticated audio processing chain:

1. **Audio Capture**: Platform-specific audio input via CPAL
2. **Noise Suppression**: Frequency-domain noise reduction
3. **Echo Cancellation**: Adaptive echo removal
4. **Opus Compression**: High-quality audio encoding
5. **Encryption**: ChaCha20-Poly1305 authenticated encryption
6. **Network Transport**: UDP-based real-time delivery

## Security Features

- **Forward Secrecy**: Keys are rotated regularly and previous keys cannot decrypt future messages
- **Replay Protection**: Nonce-based protection against message replay attacks
- **Authenticated Encryption**: ChaCha20-Poly1305 AEAD ensures integrity and confidentiality
- **Key Exchange**: X25519 elliptic curve Diffie-Hellman for secure key agreement

## Configuration

Configuration is managed through TOML files stored in platform-specific directories:

- **Linux**: `~/.config/humr/config.toml`
- **macOS**: `~/Library/Application Support/humr/config.toml`
- **Windows**: `%APPDATA%/humr/config.toml`

Example configuration:

```toml
[audio]
sample_rate = 48000
buffer_size = 1024
noise_suppression = true
echo_cancellation = true

[network]
port = 8080
max_connections = 10

[security]
key_rotation_interval = 3600  # seconds
```

## Development

### Running Tests

```bash
# Run all tests
cargo test

# Run specific test categories
cargo test security_tests
cargo test audio_tests
cargo test integration_tests

# Run with output
cargo test -- --nocapture
```

### Code Quality

```bash
# Check compilation
cargo check

# Run linter
cargo clippy

# Format code
cargo fmt
```

## Architecture

Humr is built with a clean, modular architecture:

- **`audio.rs`**: Audio capture, processing, and playback
- **`realtime_audio.rs`**: Lock-free real-time audio pipeline
- **`opus_codec.rs`**: Opus audio compression/decompression
- **`noise_suppression.rs`**: Frequency-domain noise reduction
- **`echo_cancellation.rs`**: Adaptive echo cancellation
- **`security.rs`**: Cryptographic operations and key management
- **`networking.rs`**: UDP-based communication protocol
- **`monitoring.rs`**: Performance metrics and health monitoring
- **`error_recovery.rs`**: Graceful error handling and recovery
- **`ui.rs`**: User interface and controls

## Performance

Humr is designed for real-time performance:

- **Latency**: < 20ms end-to-end processing
- **CPU Usage**: < 5% on modern hardware
- **Memory**: < 50MB runtime footprint
- **Network**: Adaptive bitrate 16-128 kbps

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Follow Rust naming conventions and idioms
- Write comprehensive tests for new features
- Maintain real-time performance requirements
- Document public APIs thoroughly
- Use `cargo fmt` and `cargo clippy` before committing

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Built with [CPAL](https://github.com/RustAudio/cpal) for cross-platform audio
- Uses [Ring](https://github.com/briansmith/ring) for cryptographic operations
- Opus codec via [audiopus](https://github.com/lakelezz/audiopus)
- Real-time DSP with [dasp](https://github.com/RustAudio/dasp)

## Support

For questions, issues, or contributions:

- [GitHub Issues](https://github.com/your-username/humr/issues)
- [Documentation](docs/)
- [Security Policy](SECURITY.md)