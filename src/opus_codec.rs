use anyhow::{Result, anyhow};
use log::{info, warn, error};
use audiopus::{coder::Encoder, coder::Decoder, Channels, Application, SampleRate, Bitrate};
use crate::realtime_audio::{AudioFrame, SAMPLE_RATE, CHANNELS, FRAME_SIZE_SAMPLES, FRAME_SIZE_SAMPLES_PER_CHANNEL};

/// Opus codec configuration for voice communication
#[derive(Debug, Clone)]
pub struct OpusConfig {
    /// Sample rate (must match AudioFrame configuration)
    pub sample_rate: u32,
    /// Number of channels (must match AudioFrame configuration)
    pub channels: u16,
    /// Target bitrate in bits per second
    pub bitrate: u32,
    /// Application type (VoIP optimized)
    pub application: OpusApplication,
    /// Frame duration in milliseconds
    pub frame_duration_ms: u32,
    /// Complexity (0-10, higher = better quality but more CPU)
    pub complexity: u32,
    /// Frame size in milliseconds
    pub frame_size_ms: u32,
    /// Forward Error Correction enabled
    pub fec_enabled: bool,
    /// Discontinuous Transmission enabled
    pub dtx_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum OpusApplication {
    VoIP,
    Audio,
    LowDelay,
}

impl Default for OpusConfig {
    fn default() -> Self {
        Self {
            sample_rate: SAMPLE_RATE,
            channels: CHANNELS,
            bitrate: 64000,  // 64 kbps - good quality for voice
            application: OpusApplication::VoIP,
            frame_duration_ms: 20,  // 20ms frames (matches AudioFrame)
            complexity: 5,   // Balanced complexity
            frame_size_ms: 20,
            fec_enabled: true,  // Enable FEC for better error resilience
            dtx_enabled: false, // Disable DTX for consistent quality
        }
    }
}

impl OpusConfig {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        if self.sample_rate != SAMPLE_RATE {
            return Err(anyhow!("Sample rate must be {}", SAMPLE_RATE));
        }
        if self.channels != CHANNELS {
            return Err(anyhow!("Channels must be {}", CHANNELS));
        }
        if self.bitrate < 6000 || self.bitrate > 510000 {
            return Err(anyhow!("Bitrate must be between 6000 and 510000"));
        }
        if self.complexity > 10 {
            return Err(anyhow!("Complexity must be <= 10"));
        }
        if ![10, 20, 40, 60].contains(&self.frame_duration_ms) {
            return Err(anyhow!("Frame duration must be 10, 20, 40, or 60 ms"));
        }
        Ok(())
    }
}

/// High-quality Opus audio codec for voice communication
pub struct OpusCodec {
    config: OpusConfig,
    encoder: Encoder,
    decoder: Decoder,
    encoded_buffer: Vec<u8>,
    decoded_buffer_i16: Vec<i16>,
    frames_encoded: u64,
    frames_decoded: u64,
    encoding_errors: u64,
    decoding_errors: u64,
    total_bytes_encoded: u64,
}

impl OpusCodec {
    /// Create new Opus codec with specified configuration
    pub fn new(config: OpusConfig) -> Result<Self> {
        info!("Creating Opus codec: {}Hz, {} channels, {} kbps",
              config.sample_rate, config.channels, config.bitrate / 1000);

        // Validate configuration matches AudioFrame constants
        if config.sample_rate != SAMPLE_RATE {
            return Err(anyhow!("Opus sample rate ({}) must match AudioFrame sample rate ({})",
                              config.sample_rate, SAMPLE_RATE));
        }
        if config.channels != CHANNELS {
            return Err(anyhow!("Opus channels ({}) must match AudioFrame channels ({})",
                              config.channels, CHANNELS));
        }

        // Convert to audiopus types
        let opus_sample_rate = match config.sample_rate {
            8000 => SampleRate::Hz8000,
            12000 => SampleRate::Hz12000,
            16000 => SampleRate::Hz16000,
            24000 => SampleRate::Hz24000,
            48000 => SampleRate::Hz48000,
            _ => return Err(anyhow!("Unsupported sample rate: {}", config.sample_rate)),
        };

        let opus_channels = match config.channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            _ => return Err(anyhow!("Unsupported channel count: {}", config.channels)),
        };

        let opus_application = match config.application {
            OpusApplication::VoIP => Application::Voip,
            OpusApplication::Audio => Application::Audio,
            OpusApplication::LowDelay => Application::LowDelay,
        };

        // Create encoder
        let mut encoder = Encoder::new(opus_sample_rate, opus_channels, opus_application)
            .map_err(|e| anyhow!("Failed to create Opus encoder: {}", e))?;

        // Configure encoder settings
        encoder.set_bitrate(Bitrate::BitsPerSecond(config.bitrate as i32))
            .map_err(|e| anyhow!("Failed to set Opus bitrate: {}", e))?;

        encoder.set_complexity(config.complexity as u8)
            .map_err(|e| anyhow!("Failed to set Opus complexity: {}", e))?;

        // Enable VBR for better quality
        encoder.set_vbr(true)
            .map_err(|e| anyhow!("Failed to enable Opus VBR: {}", e))?;

        // Create decoder
        let decoder = Decoder::new(opus_sample_rate, opus_channels)
            .map_err(|e| anyhow!("Failed to create Opus decoder: {}", e))?;

        // Pre-allocate buffers
        let max_encoded_size = 4000; // Opus max packet size
        let decoded_buffer_size = FRAME_SIZE_SAMPLES; // Already includes both channels

        info!("Opus codec created successfully");

        Ok(Self {
            config,
            encoder,
            decoder,
            encoded_buffer: vec![0u8; max_encoded_size],
            decoded_buffer_i16: vec![0i16; decoded_buffer_size],
            frames_encoded: 0,
            frames_decoded: 0,
            encoding_errors: 0,
            decoding_errors: 0,
            total_bytes_encoded: 0,
        })
    }

    /// Encode audio frame to compressed data
    pub fn encode(&mut self, frame: &AudioFrame) -> Result<Vec<u8>> {
        // Convert f32 samples to i16 for Opus (Opus expects 16-bit samples)
        let mut i16_samples = Vec::with_capacity(frame.samples.len());
        for &sample in &frame.samples {
            // Clamp and convert to i16 range
            let clamped = sample.clamp(-1.0, 1.0);
            let i16_sample = (clamped * 32767.0) as i16;
            i16_samples.push(i16_sample);
        }

        // Encode with Opus
        match self.encoder.encode(&i16_samples, &mut self.encoded_buffer) {
            Ok(encoded_len) => {
                self.frames_encoded += 1;
                self.total_bytes_encoded += encoded_len as u64;
                // Return only the used portion of the buffer
                Ok(self.encoded_buffer[..encoded_len].to_vec())
            }
            Err(e) => {
                self.encoding_errors += 1;
                error!("Opus encoding failed: {}", e);
                Err(anyhow!("Opus encoding failed: {}", e))
            }
        }
    }

    /// Decode compressed data to audio frame
    pub fn decode(&mut self, encoded_data: &[u8]) -> Result<AudioFrame> {
        use audiopus::{packet::Packet, MutSignals};

        // Create packet wrapper
        let packet = Packet::try_from(encoded_data)
            .map_err(|e| anyhow!("Failed to create Opus packet: {}", e))?;

        // Create mutable signals wrapper for i16 buffer
        let mut signals = MutSignals::try_from(&mut self.decoded_buffer_i16[..])
            .map_err(|e| anyhow!("Failed to create signals wrapper: {}", e))?;

        // Decode with Opus
        let decoded_len = match self.decoder.decode(Some(packet), signals, false) {
            Ok(len) => {
                self.frames_decoded += 1;
                len
            },
            Err(e) => {
                self.decoding_errors += 1;
                error!("Opus decoding failed: {}", e);
                return Err(anyhow!("Opus decoding failed: {}", e));
            }
        };

        // Verify we got the expected number of samples (Opus returns samples per channel)
        let expected_samples = FRAME_SIZE_SAMPLES_PER_CHANNEL;
        if decoded_len != expected_samples {
            warn!("Opus decoded {} samples, expected {}", decoded_len, expected_samples);
        }

        // Convert i16 samples back to f32 and create AudioFrame
        // decoded_len is samples per channel, but the buffer contains interleaved samples
        // So for stereo, we need decoded_len * channels total samples
        let total_samples = decoded_len * self.config.channels as usize;
        let mut f32_samples = Vec::with_capacity(total_samples);
        for &i16_sample in self.decoded_buffer_i16[..total_samples].iter() {
            // Convert from i16 range back to f32 range
            f32_samples.push(i16_sample as f32 / 32767.0);
        }
        let mut frame = AudioFrame::new(f32_samples);

        // Set frame metadata
        frame.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(frame)
    }

    /// Handle packet loss by generating concealment frame
    pub fn decode_lost_packet(&mut self) -> Result<AudioFrame> {
        use audiopus::MutSignals;

        info!("Generating packet loss concealment frame");

        // Create mutable signals wrapper for i16 buffer
        let mut signals = MutSignals::try_from(&mut self.decoded_buffer_i16[..])
            .map_err(|e| anyhow!("Failed to create signals wrapper: {}", e))?;

        // Opus can generate concealment for lost packets
        let decoded_len = match self.decoder.decode(None, signals, false) {
            Ok(len) => len,
            Err(e) => {
                error!("Opus packet loss concealment failed: {}", e);
                return Ok(AudioFrame::silence()); // Fallback to silence
            }
        };

        // Convert i16 samples back to f32 and create AudioFrame
        let mut f32_samples = Vec::with_capacity(decoded_len);
        for &i16_sample in self.decoded_buffer_i16[..decoded_len].iter() {
            // Convert from i16 range back to f32 range
            f32_samples.push(i16_sample as f32 / 32767.0);
        }
        let mut frame = AudioFrame::new(f32_samples);

        frame.timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Ok(frame)
    }

    /// Update codec bitrate dynamically
    pub fn set_bitrate(&mut self, bitrate: u32) -> Result<()> {
        info!("Updating Opus bitrate: {} -> {} kbps",
              self.config.bitrate / 1000, bitrate / 1000);

        self.encoder.set_bitrate(Bitrate::BitsPerSecond(bitrate as i32))
            .map_err(|e| anyhow!("Failed to update Opus bitrate: {}", e))?;

        self.config.bitrate = bitrate;
        Ok(())
    }

    /// Update codec complexity (CPU vs quality tradeoff)
    pub fn set_complexity(&mut self, complexity: u32) -> Result<()> {
        if complexity > 10 {
            return Err(anyhow!("Opus complexity must be 0-10, got {}", complexity));
        }

        info!("Updating Opus complexity: {} -> {}", self.config.complexity, complexity);

        self.encoder.set_complexity(complexity as u8)
            .map_err(|e| anyhow!("Failed to update Opus complexity: {}", e))?;

        self.config.complexity = complexity;
        Ok(())
    }

    /// Get current codec configuration
    pub fn get_config(&self) -> &OpusConfig {
        &self.config
    }

    /// Get codec statistics
    pub fn get_stats(&self) -> OpusStats {
        let average_compression_ratio = if self.frames_encoded > 0 {
            let uncompressed_bytes = self.frames_encoded * FRAME_SIZE_SAMPLES as u64 * 4; // 4 bytes per f32
            uncompressed_bytes as f64 / self.total_bytes_encoded as f64
        } else {
            0.0
        };

        OpusStats {
            sample_rate: self.config.sample_rate,
            channels: self.config.channels,
            bitrate: self.config.bitrate,
            complexity: self.config.complexity,
            frame_duration_ms: self.config.frame_duration_ms,
            frames_encoded: self.frames_encoded,
            frames_decoded: self.frames_decoded,
            encoding_errors: self.encoding_errors,
            decoding_errors: self.decoding_errors,
            total_bytes_encoded: self.total_bytes_encoded,
            average_compression_ratio,
        }
    }

    /// Reset codec state (useful after connection drops)
    pub fn reset(&mut self) -> Result<()> {
        info!("Resetting Opus codec state");

        // Recreate encoder and decoder to reset internal state
        let opus_sample_rate = match self.config.sample_rate {
            8000 => SampleRate::Hz8000,
            12000 => SampleRate::Hz12000,
            16000 => SampleRate::Hz16000,
            24000 => SampleRate::Hz24000,
            48000 => SampleRate::Hz48000,
            _ => return Err(anyhow!("Unsupported sample rate: {}", self.config.sample_rate)),
        };

        let opus_channels = match self.config.channels {
            1 => Channels::Mono,
            2 => Channels::Stereo,
            _ => return Err(anyhow!("Unsupported channel count: {}", self.config.channels)),
        };

        let opus_application = match self.config.application {
            OpusApplication::VoIP => Application::Voip,
            OpusApplication::Audio => Application::Audio,
            OpusApplication::LowDelay => Application::LowDelay,
        };

        // Recreate encoder
        let mut encoder = Encoder::new(opus_sample_rate, opus_channels, opus_application)
            .map_err(|e| anyhow!("Failed to recreate Opus encoder: {}", e))?;

        // Restore settings
        encoder.set_bitrate(Bitrate::BitsPerSecond(self.config.bitrate as i32))
            .map_err(|e| anyhow!("Failed to restore Opus bitrate: {}", e))?;
        encoder.set_complexity(self.config.complexity as u8)
            .map_err(|e| anyhow!("Failed to restore Opus complexity: {}", e))?;
        encoder.set_vbr(true)
            .map_err(|e| anyhow!("Failed to restore Opus VBR: {}", e))?;

        // Recreate decoder
        let decoder = Decoder::new(opus_sample_rate, opus_channels)
            .map_err(|e| anyhow!("Failed to recreate Opus decoder: {}", e))?;

        self.encoder = encoder;
        self.decoder = decoder;

        Ok(())
    }
}

/// Opus codec statistics
#[derive(Debug, Clone)]
pub struct OpusStats {
    pub sample_rate: u32,
    pub channels: u16,
    pub bitrate: u32,
    pub complexity: u32,
    pub frame_duration_ms: u32,
    pub frames_encoded: u64,
    pub frames_decoded: u64,
    pub encoding_errors: u64,
    pub decoding_errors: u64,
    pub total_bytes_encoded: u64,
    pub average_compression_ratio: f64,
}

impl OpusStats {
    pub fn compression_ratio(&self) -> f64 {
        // Calculate theoretical compression ratio
        let uncompressed_bps = self.sample_rate * self.channels as u32 * 32; // 32-bit float
        uncompressed_bps as f64 / self.bitrate as f64
    }
}