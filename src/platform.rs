use anyhow::{Result, anyhow};
use cpal::{Device, Host, Stream, StreamConfig, SampleRate};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::Arc;
use crossbeam::queue::SegQueue;

// Cross-platform audio adapter using CPAL
pub struct PlatformAudioAdapter {
    host: Host,
    input_device: Option<Device>,
    output_device: Option<Device>,
    input_config: Option<StreamConfig>,
    output_config: Option<StreamConfig>,
    input_stream: Option<Stream>,
    output_stream: Option<Stream>,
    // Lock-free queues for audio data
    input_queue: Arc<SegQueue<i16>>,
    output_queue: Arc<SegQueue<i16>>,
}

impl PlatformAudioAdapter {
    pub fn new() -> Self {
        let host = cpal::default_host();
        Self {
            host,
            input_device: None,
            output_device: None,
            input_config: None,
            output_config: None,
            input_stream: None,
            output_stream: None,
            input_queue: Arc::new(SegQueue::new()),
            output_queue: Arc::new(SegQueue::new()),
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        // Initialize input device (microphone)
        self.input_device = Some(
            self.host
                .default_input_device()
                .ok_or_else(|| anyhow!("No input device available"))?
        );

        // Initialize output device (speakers)
        self.output_device = Some(
            self.host
                .default_output_device()
                .ok_or_else(|| anyhow!("No output device available"))?
        );

        // Configure input stream
        if let Some(ref input_device) = self.input_device {
            let _input_config = input_device
                .default_input_config()
                .map_err(|e| anyhow!("Failed to get input config: {}", e))?;

            self.input_config = Some(StreamConfig {
                channels: 1, // Mono input
                sample_rate: SampleRate(48000),
                buffer_size: cpal::BufferSize::Default,
            });
        }

        // Configure output stream
        if let Some(ref output_device) = self.output_device {
            let _output_config = output_device
                .default_output_config()
                .map_err(|e| anyhow!("Failed to get output config: {}", e))?;

            self.output_config = Some(StreamConfig {
                channels: 1, // Mono output
                sample_rate: SampleRate(48000),
                buffer_size: cpal::BufferSize::Default,
            });
        }

        Ok(())
    }

    pub fn capture_audio_frame(&self, buffer: &mut [i16]) -> Result<usize> {
        // Read from input queue (populated by input stream callback)
        let mut samples_read = 0;
        for sample in buffer.iter_mut() {
            if let Some(audio_sample) = self.input_queue.pop() {
                *sample = audio_sample;
                samples_read += 1;
            } else {
                // No more data available, fill with silence
                *sample = 0;
            }
        }
        Ok(samples_read)
    }

    pub fn playback_audio_frame(&self, buffer: &[i16]) -> Result<usize> {
        // Write to output queue (consumed by output stream callback)
        for &sample in buffer.iter() {
            self.output_queue.push(sample);
        }
        Ok(buffer.len())
    }

    pub fn get_input_devices(&self) -> Vec<String> {
        self.host
            .input_devices()
            .map(|devices| {
                devices
                    .filter_map(|device| device.name().ok())
                    .collect()
            })
            .unwrap_or_else(|_| vec!["default".to_string()])
    }

    pub fn get_output_devices(&self) -> Vec<String> {
        self.host
            .output_devices()
            .map(|devices| {
                devices
                    .filter_map(|device| device.name().ok())
                    .collect()
            })
            .unwrap_or_else(|_| vec!["default".to_string()])
    }

    /// Start the audio streams for input and output
    pub fn start_streams(&mut self) -> Result<()> {
        // Start input stream
        if let (Some(device), Some(config)) = (&self.input_device, &self.input_config) {
            let input_queue = Arc::clone(&self.input_queue);

            let input_stream = device.build_input_stream(
                config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    // Convert f32 samples to i16 and push to queue
                    for &sample in data.iter() {
                        let sample_i16 = (sample * i16::MAX as f32) as i16;
                        input_queue.push(sample_i16);
                    }
                },
                move |err| {
                    eprintln!("Audio input stream error: {}", err);
                },
                None,
            ).map_err(|e| anyhow!("Failed to build input stream: {}", e))?;

            input_stream.play().map_err(|e| anyhow!("Failed to start input stream: {}", e))?;
            self.input_stream = Some(input_stream);
        }

        // Start output stream
        if let (Some(device), Some(config)) = (&self.output_device, &self.output_config) {
            let output_queue = Arc::clone(&self.output_queue);

            let output_stream = device.build_output_stream(
                config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    // Pop samples from queue and convert i16 to f32
                    for sample in data.iter_mut() {
                        if let Some(sample_i16) = output_queue.pop() {
                            *sample = sample_i16 as f32 / i16::MAX as f32;
                        } else {
                            *sample = 0.0; // Silence if no data available
                        }
                    }
                },
                move |err| {
                    eprintln!("Audio output stream error: {}", err);
                },
                None,
            ).map_err(|e| anyhow!("Failed to build output stream: {}", e))?;

            output_stream.play().map_err(|e| anyhow!("Failed to start output stream: {}", e))?;
            self.output_stream = Some(output_stream);
        }

        Ok(())
    }

    /// Stop the audio streams
    pub fn stop_streams(&mut self) {
        if let Some(stream) = self.input_stream.take() {
            let _ = stream.pause();
        }
        if let Some(stream) = self.output_stream.take() {
            let _ = stream.pause();
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) -> Result<()> {
        // Update configuration for next stream initialization
        if let Some(ref mut config) = self.input_config {
            config.sample_rate = SampleRate(sample_rate);
        }
        if let Some(ref mut config) = self.output_config {
            config.sample_rate = SampleRate(sample_rate);
        }
        Ok(())
    }

    pub fn set_buffer_size(&mut self, buffer_size: u32) -> Result<()> {
        // Update configuration for next stream initialization
        let buffer_size_config = cpal::BufferSize::Fixed(buffer_size);
        if let Some(ref mut config) = self.input_config {
            config.buffer_size = buffer_size_config;
        }
        if let Some(ref mut config) = self.output_config {
            config.buffer_size = buffer_size_config;
        }
        Ok(())
    }
}