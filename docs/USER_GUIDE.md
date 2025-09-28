# Humr User Guide

## Getting Started

### First Time Setup

1. **Install Humr**: Follow the installation instructions in the [README](../README.md)
2. **Audio Device Check**: Ensure you have a working microphone and speakers/headphones
3. **Initial Configuration**: Humr will create a default configuration file on first run

### Basic Usage

#### Starting a Voice Session

```bash
# Start Humr
cargo run

# Or if installed system-wide
humr
```

When Humr starts, it will:
- Initialize audio devices
- Load configuration
- Set up secure communication
- Display connection status

#### Connecting to Others

Humr uses a peer-to-peer model for voice communication:

1. **Share your IP and port** with the person you want to talk to
2. **Get their IP and port** for connection
3. **Establish connection** through the interface

## Audio Configuration

### Microphone Settings

- **Gain Control**: Adjust input sensitivity
- **Noise Gate**: Set threshold to filter background noise
- **Push-to-Talk**: Enable/disable always-on microphone

### Speaker Settings

- **Volume Control**: Adjust output volume
- **Echo Cancellation**: Enable/disable (recommended: on)
- **Audio Quality**: Choose between bandwidth and quality

### Audio Quality Presets

| Preset | Bitrate | Quality | CPU Usage | Use Case |
|--------|---------|---------|-----------|----------|
| Low | 16 kbps | Basic | Minimal | Poor connections |
| Medium | 32 kbps | Good | Low | Normal use |
| High | 64 kbps | Excellent | Medium | High quality |
| Ultra | 128 kbps | Studio | High | Professional |

## Configuration Options

### Audio Settings (`config.toml`)

```toml
[audio]
# Sample rate (Hz) - higher = better quality, more CPU
sample_rate = 48000

# Buffer size (samples) - lower = less latency, more CPU
buffer_size = 1024

# Enable advanced audio processing
noise_suppression = true
echo_cancellation = true
automatic_gain_control = true

# Audio quality preset
quality_preset = "medium"  # low, medium, high, ultra

# Microphone settings
mic_gain = 1.0            # 0.0 to 2.0
noise_gate_threshold = -40 # dB, lower = more sensitive
push_to_talk = false      # true for push-to-talk mode
ptt_key = "Space"         # key for push-to-talk
```

### Network Settings

```toml
[network]
# Port for incoming connections
port = 8080

# Maximum simultaneous connections
max_connections = 10

# Connection timeout (seconds)
timeout = 30

# Automatic port forwarding (UPnP)
upnp_enabled = true
```

### Security Settings

```toml
[security]
# Key rotation interval (seconds)
key_rotation_interval = 3600

# Enable forward secrecy
forward_secrecy = true

# Require authentication for connections
require_auth = true

# Allowed connection sources (IP addresses)
allowed_ips = ["0.0.0.0/0"]  # Allow all by default
```

## Troubleshooting

### Common Issues

#### No Audio Input/Output

**Problem**: Microphone or speakers not working
**Solutions**:
1. Check audio device permissions
2. Verify device is not in use by another application
3. Restart Humr
4. Check system audio settings

#### High Latency

**Problem**: Noticeable delay in voice transmission
**Solutions**:
1. Reduce buffer size: `buffer_size = 512` or `buffer_size = 256`
2. Close other audio applications
3. Use wired internet connection
4. Reduce audio quality preset

#### Connection Issues

**Problem**: Cannot connect to other users
**Solutions**:
1. Check firewall settings (allow port 8080)
2. Verify IP addresses are correct
3. Enable UPnP in router settings
4. Use direct IP connection instead of hostname

#### Audio Quality Issues

**Problem**: Poor voice quality or artifacts
**Solutions**:
1. Increase quality preset
2. Check microphone positioning
3. Enable noise suppression
4. Reduce background noise
5. Check network bandwidth

### Performance Optimization

#### Low-End Hardware

For devices with limited CPU:
```toml
[audio]
sample_rate = 24000
buffer_size = 2048
quality_preset = "low"
noise_suppression = false
echo_cancellation = false
```

#### High-End Hardware

For powerful devices:
```toml
[audio]
sample_rate = 48000
buffer_size = 256
quality_preset = "ultra"
noise_suppression = true
echo_cancellation = true
automatic_gain_control = true
```

## Advanced Features

### Push-to-Talk Mode

Enable push-to-talk for better noise control:

```toml
[audio]
push_to_talk = true
ptt_key = "Space"  # or "F1", "Ctrl", etc.
```

### Multiple Connections

Connect to multiple users simultaneously:
- Each connection maintains independent audio streams
- CPU usage scales with number of connections
- Configure `max_connections` based on hardware

### Audio Recording

Record conversations (with permission):
```toml
[recording]
enabled = true
output_directory = "~/humr_recordings"
format = "opus"  # opus, wav, flac
```

## Security Best Practices

### Connection Security

1. **Verify Connections**: Always verify you're connecting to the intended person
2. **Use Strong Passwords**: If authentication is enabled
3. **Keep Software Updated**: Regularly update Humr for security patches
4. **Monitor Connections**: Check who's connected in the interface

### Privacy Considerations

1. **Local Recordings**: Be aware recordings are stored locally
2. **Key Rotation**: Shorter intervals = better security (more CPU usage)
3. **Network Monitoring**: Voice traffic is encrypted but metadata may be visible
4. **Firewall Configuration**: Only open necessary ports

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| Space | Push-to-talk (if enabled) |
| M | Mute/unmute microphone |
| V | Adjust volume |
| Q | Quit application |
| C | Show connections |
| S | Show statistics |

## Getting Help

If you encounter issues:

1. **Check Logs**: Run with `RUST_LOG=debug` for detailed information
2. **Configuration**: Verify your `config.toml` settings
3. **Hardware**: Test your audio devices with other applications
4. **Network**: Check firewall and router settings
5. **Community**: Report issues on GitHub

For immediate help:
```bash
# Show version and build info
humr --version

# Test audio devices
humr --test-audio

# Validate configuration
humr --check-config
```