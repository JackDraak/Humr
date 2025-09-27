use anyhow::{Result, anyhow};
use log::{info, error, warn};
use ringbuf::{HeapRb, traits::*};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use std::thread::{self, JoinHandle};
use std::time::Duration;
use cpal::{Device, Stream, StreamConfig, SampleRate, BufferSize};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

/// Audio frame size in samples per channel (20ms at 48kHz = 960 samples per channel)
pub const FRAME_SIZE_SAMPLES_PER_CHANNEL: usize = 960;
/// Total samples for stereo frame
pub const FRAME_SIZE_SAMPLES: usize = FRAME_SIZE_SAMPLES_PER_CHANNEL * CHANNELS as usize;
/// Frame duration in milliseconds
pub const FRAME_SIZE_MS: u32 = 20;
/// Audio sample rate (48kHz for high quality)
pub const SAMPLE_RATE: u32 = 48000;
/// Audio channels (stereo)
pub const CHANNELS: u16 = 2;
/// Ring buffer capacity (store ~500ms of audio)
pub const RING_BUFFER_CAPACITY: usize = FRAME_SIZE_SAMPLES * 25;

/// Real-time safe audio frame container
#[derive(Debug, Clone)]
pub struct AudioFrame {
    pub samples: Vec<f32>,
    pub timestamp: u64,
    pub sequence: u32,
}

/// Zero-copy audio buffer for efficient memory management
pub struct ZeroCopyAudioBuffer {
    /// Pre-allocated memory pool for audio frames
    frame_pool: Vec<AudioFrame>,
    /// Available frame indices (acts as a free list)
    free_indices: std::collections::VecDeque<usize>,
    /// Pool capacity
    capacity: usize,
}

impl ZeroCopyAudioBuffer {
    /// Create new zero-copy audio buffer pool
    pub fn new(capacity: usize) -> Self {
        info!("Creating zero-copy audio buffer pool with {} frames", capacity);

        // Pre-allocate all frames
        let mut frame_pool = Vec::with_capacity(capacity);
        for i in 0..capacity {
            frame_pool.push(AudioFrame {
                samples: vec![0.0; FRAME_SIZE_SAMPLES * CHANNELS as usize],
                timestamp: 0,
                sequence: i as u32,
            });
        }

        // Initialize free list with all indices
        let mut free_indices = std::collections::VecDeque::with_capacity(capacity);
        for i in 0..capacity {
            free_indices.push_back(i);
        }

        Self {
            frame_pool,
            free_indices,
            capacity,
        }
    }

    /// Acquire a frame from the pool (zero-copy)
    pub fn acquire_frame(&mut self) -> Option<&mut AudioFrame> {
        if let Some(index) = self.free_indices.pop_front() {
            // Return mutable reference to pre-allocated frame
            Some(&mut self.frame_pool[index])
        } else {
            // Pool exhausted
            None
        }
    }

    /// Release a frame back to the pool
    pub fn release_frame(&mut self, frame: &AudioFrame) {
        // Find the frame's index in the pool
        let frame_ptr = frame as *const AudioFrame;
        let pool_start = self.frame_pool.as_ptr();

        // Calculate index from pointer arithmetic
        let index = unsafe {
            (frame_ptr.offset_from(pool_start)) as usize
        };

        if index < self.capacity {
            // Clear frame data for reuse
            // Safety: We know this index is valid
            unsafe {
                let frame_mut = &mut *self.frame_pool.as_mut_ptr().add(index);
                frame_mut.samples.fill(0.0);
                frame_mut.timestamp = 0;
            }

            // Return to free list
            self.free_indices.push_back(index);
        }
    }

    /// Get pool statistics
    pub fn get_stats(&self) -> ZeroCopyBufferStats {
        ZeroCopyBufferStats {
            total_capacity: self.capacity,
            available_frames: self.free_indices.len(),
            allocated_frames: self.capacity - self.free_indices.len(),
        }
    }
}

/// Zero-copy buffer statistics
#[derive(Debug, Clone)]
pub struct ZeroCopyBufferStats {
    pub total_capacity: usize,
    pub available_frames: usize,
    pub allocated_frames: usize,
}

impl ZeroCopyBufferStats {
    pub fn utilization_percent(&self) -> f32 {
        if self.total_capacity > 0 {
            (self.allocated_frames as f32 / self.total_capacity as f32) * 100.0
        } else {
            0.0
        }
    }
}

impl AudioFrame {
    pub fn new(samples: Vec<f32>) -> Self {
        Self {
            samples,
            timestamp: 0,
            sequence: 0,
        }
    }

    pub fn empty() -> Self {
        Self {
            samples: vec![0.0; FRAME_SIZE_SAMPLES * CHANNELS as usize],
            timestamp: 0,
            sequence: 0,
        }
    }

    pub fn silence() -> Self {
        Self::empty()
    }

    pub fn len(&self) -> usize {
        self.samples.len()
    }

    pub fn channels(&self) -> u16 {
        CHANNELS
    }

    pub fn sample_rate(&self) -> u32 {
        SAMPLE_RATE
    }
}

/// Real-time audio processor with lock-free architecture
pub struct RealTimeAudioProcessor {
    // Audio device streams
    input_stream: Option<Stream>,
    output_stream: Option<Stream>,

    // Control flags
    is_running: Arc<AtomicBool>,
    processing_thread: Option<JoinHandle<()>>,

    // Audio devices
    input_device: Option<Device>,
    output_device: Option<Device>,

    // Statistics and monitoring
    frames_processed: Arc<std::sync::atomic::AtomicU64>,
    last_input_time: Arc<std::sync::atomic::AtomicU64>,
    last_output_time: Arc<std::sync::atomic::AtomicU64>,

    // Ring buffer handles for processing thread
    input_consumer: Option<ringbuf::HeapCons<AudioFrame>>,
    output_producer: Option<ringbuf::HeapProd<AudioFrame>>,
}

impl RealTimeAudioProcessor {
    /// Create new real-time audio processor
    pub fn new() -> Result<Self> {
        info!("Initializing real-time audio processor");

        Ok(Self {
            input_stream: None,
            output_stream: None,
            is_running: Arc::new(AtomicBool::new(false)),
            processing_thread: None,
            input_device: None,
            output_device: None,
            frames_processed: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            last_input_time: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            last_output_time: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            input_consumer: None,
            output_producer: None,
        })
    }

    /// Initialize audio devices and streams
    pub fn initialize(&mut self) -> Result<()> {
        info!("Initializing audio devices");

        let host = cpal::default_host();

        // Get default input and output devices
        let input_device = host.default_input_device()
            .ok_or_else(|| anyhow!("No default input device available"))?;
        let output_device = host.default_output_device()
            .ok_or_else(|| anyhow!("No default output device available"))?;

        info!("Input device: {}", input_device.name().unwrap_or("Unknown".to_string()));
        info!("Output device: {}", output_device.name().unwrap_or("Unknown".to_string()));

        // Configure audio streams
        let config = StreamConfig {
            channels: CHANNELS,
            sample_rate: SampleRate(SAMPLE_RATE),
            buffer_size: BufferSize::Fixed(FRAME_SIZE_SAMPLES as u32),
        };

        // Create lock-free ring buffers
        let input_rb = HeapRb::<AudioFrame>::new(RING_BUFFER_CAPACITY);
        let (input_producer, input_consumer) = input_rb.split();

        let output_rb = HeapRb::<AudioFrame>::new(RING_BUFFER_CAPACITY);
        let (output_producer, output_consumer) = output_rb.split();

        // Store the consumer/producer for processing thread
        self.input_consumer = Some(input_consumer);
        self.output_producer = Some(output_producer);

        // Clone atomic counters for callbacks
        let frames_processed_clone = self.frames_processed.clone();
        let last_input_time_clone = self.last_input_time.clone();
        let last_output_time_clone = self.last_output_time.clone();

        // Create input stream with owned producer
        let mut input_producer = input_producer; // Make mutable
        let input_stream = input_device.build_input_stream(
            &config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                Self::input_callback(data, &mut input_producer, &frames_processed_clone, &last_input_time_clone);
            },
            |err| {
                error!("Audio input stream error: {}", err);
            },
            None,
        )?;

        // Create output stream with owned consumer
        let mut output_consumer = output_consumer; // Make mutable
        let output_stream = output_device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                Self::output_callback(data, &mut output_consumer, &last_output_time_clone);
            },
            |err| {
                error!("Audio output stream error: {}", err);
            },
            None,
        )?;

        self.input_device = Some(input_device);
        self.output_device = Some(output_device);
        self.input_stream = Some(input_stream);
        self.output_stream = Some(output_stream);

        Ok(())
    }

    /// Start audio processing
    pub fn start(&mut self) -> Result<()> {
        info!("Starting real-time audio processing");

        if self.is_running.load(Ordering::Relaxed) {
            return Err(anyhow!("Audio processor already running"));
        }

        // Start audio streams
        if let Some(input_stream) = &self.input_stream {
            input_stream.play()?;
        }
        if let Some(output_stream) = &self.output_stream {
            output_stream.play()?;
        }

        // Start processing thread
        self.is_running.store(true, Ordering::Relaxed);
        let is_running_clone = self.is_running.clone();

        // Move ring buffer components to processing thread
        let input_consumer = self.input_consumer.take()
            .ok_or_else(|| anyhow!("Input consumer not initialized"))?;
        let output_producer = self.output_producer.take()
            .ok_or_else(|| anyhow!("Output producer not initialized"))?;

        let processing_thread = thread::spawn(move || {
            // Set real-time thread priority for low-latency audio processing
            Self::set_realtime_priority();
            Self::processing_loop(is_running_clone, input_consumer, output_producer);
        });

        self.processing_thread = Some(processing_thread);

        info!("Real-time audio processing started");
        Ok(())
    }

    /// Stop audio processing
    pub fn stop(&mut self) -> Result<()> {
        info!("Stopping real-time audio processing");

        self.is_running.store(false, Ordering::Relaxed);

        // Stop audio streams
        if let Some(input_stream) = &self.input_stream {
            input_stream.pause()?;
        }
        if let Some(output_stream) = &self.output_stream {
            output_stream.pause()?;
        }

        // Wait for processing thread to finish
        if let Some(thread) = self.processing_thread.take() {
            if let Err(e) = thread.join() {
                error!("Error joining processing thread: {:?}", e);
            }
        }

        info!("Real-time audio processing stopped");
        Ok(())
    }

    /// Audio input callback - runs in real-time audio thread
    fn input_callback(
        data: &[f32],
        producer: &mut ringbuf::HeapProd<AudioFrame>,
        frames_processed: &std::sync::atomic::AtomicU64,
        last_input_time: &std::sync::atomic::AtomicU64,
    ) {
        let mut frame = AudioFrame::empty();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        last_input_time.store(now, Ordering::Relaxed);

        // Copy audio data to frame (with bounds checking)
        let copy_len = std::cmp::min(data.len(), frame.samples.len());
        frame.samples[..copy_len].copy_from_slice(&data[..copy_len]);
        frame.timestamp = now;
        frame.sequence = frames_processed.fetch_add(1, Ordering::Relaxed) as u32;

        // Try to push frame to ring buffer (non-blocking)
        if producer.try_push(frame).is_err() {
            // Buffer full - this indicates processing can't keep up
            // In production, we might want to drop frames or implement backpressure
        }
    }

    /// Audio output callback - runs in real-time audio thread
    fn output_callback(
        data: &mut [f32],
        consumer: &mut ringbuf::HeapCons<AudioFrame>,
        last_output_time: &std::sync::atomic::AtomicU64,
    ) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        last_output_time.store(now, Ordering::Relaxed);

        // Try to get frame from ring buffer
        if let Some(frame) = consumer.try_pop() {
            // Copy frame data to output buffer
            let copy_len = std::cmp::min(data.len(), frame.samples.len());
            data[..copy_len].copy_from_slice(&frame.samples[..copy_len]);
        } else {
            // No frame available - output silence
            data.fill(0.0);
        }
    }

    /// Set real-time scheduling priority for audio thread
    fn set_realtime_priority() {
        #[cfg(target_os = "linux")]
        {

            // Attempt to set SCHED_FIFO with high priority
            let result = unsafe {
                let param = libc::sched_param {
                    sched_priority: 80, // High priority (1-99 range on Linux)
                };
                libc::sched_setscheduler(0, libc::SCHED_FIFO, &param)
            };

            if result == 0 {
                info!("Successfully set real-time scheduling (SCHED_FIFO, priority 80)");
            } else {
                warn!("Failed to set real-time scheduling: {}. Running with normal priority.",
                      std::io::Error::last_os_error());
                warn!("For best performance, run with sudo or configure realtime limits");
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS uses different real-time scheduling approach
            // Set high thread priority using Mach APIs
            let result = unsafe {
                let thread = libc::pthread_self();
                let mut policy: i32 = 0;
                let mut param = libc::sched_param { sched_priority: 0 };

                // Get current policy first
                if libc::pthread_getschedparam(thread, &mut policy, &mut param) == 0 {
                    // Try to set high priority within current policy
                    param.sched_priority = 63; // High priority for macOS
                    if libc::pthread_setschedparam(thread, policy, &param) == 0 {
                        0
                    } else {
                        -1
                    }
                } else {
                    -1
                }
            };

            if result == 0 {
                info!("Successfully set high thread priority on macOS");
            } else {
                warn!("Failed to set thread priority on macOS: {}. Running with normal priority.",
                      std::io::Error::last_os_error());
            }
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            warn!("Real-time scheduling not implemented for this platform");
        }
    }

    /// Main audio processing loop - runs in dedicated thread
    fn processing_loop(
        is_running: Arc<AtomicBool>,
        mut input_consumer: ringbuf::HeapCons<AudioFrame>,
        mut output_producer: ringbuf::HeapProd<AudioFrame>,
    ) {
        info!("Audio processing loop started");

        let mut sequence_counter = 0u32;

        while is_running.load(Ordering::Relaxed) {
            // Process input frames
            while let Some(mut input_frame) = input_consumer.try_pop() {
                // TODO: This is where we'll add:
                // - Noise suppression
                // - Echo cancellation
                // - Audio compression/encoding
                // - Network packet preparation

                // For now, just pass through (placeholder for processing)
                input_frame.sequence = sequence_counter;
                sequence_counter = sequence_counter.wrapping_add(1);

                // Push processed frame to output
                if output_producer.try_push(input_frame).is_err() {
                    // Output buffer full - consider dropping frames or adjusting buffer sizes
                }
            }

            // Small sleep to prevent busy waiting
            thread::sleep(Duration::from_micros(100));
        }

        info!("Audio processing loop stopped");
    }

    /// Get audio processing statistics
    pub fn get_stats(&self) -> AudioStats {
        // Note: Buffer usage stats not available after processing thread starts
        // since ring buffer components are moved to the processing thread
        AudioStats {
            frames_processed: self.frames_processed.load(Ordering::Relaxed),
            last_input_time: self.last_input_time.load(Ordering::Relaxed),
            last_output_time: self.last_output_time.load(Ordering::Relaxed),
            input_buffer_usage: 0, // TODO: Implement via shared atomic counters if needed
            output_buffer_usage: 0, // TODO: Implement via shared atomic counters if needed
            is_running: self.is_running.load(Ordering::Relaxed),
            input_underruns: 0, // TODO: Implement underrun tracking
            output_overruns: 0, // TODO: Implement overrun tracking
        }
    }

    /// Check if processor is currently running
    pub fn is_running(&self) -> bool {
        self.is_running.load(Ordering::Relaxed)
    }

    /// Start async processing
    pub async fn start_processing(&mut self) -> Result<()> {
        if self.is_running() {
            return Err(anyhow!("Audio processor is already running"));
        }

        self.initialize()?;
        self.start()?;
        Ok(())
    }

    /// Stop async processing
    pub async fn stop_processing(&mut self) -> Result<()> {
        if !self.is_running() {
            return Err(anyhow!("Audio processor is not running"));
        }

        self.stop()?;
        Ok(())
    }

    /// Set sample rate (validate range)
    pub fn set_sample_rate(&mut self, sample_rate: u32) -> Result<()> {
        match sample_rate {
            8000 | 16000 | 22050 | 44100 | 48000 | 88200 | 96000 => {
                // Valid sample rates - would need to reinitialize streams
                Ok(())
            }
            _ => Err(anyhow!("Unsupported sample rate: {}", sample_rate))
        }
    }

    /// Set buffer size (validate power of 2 and range)
    pub fn set_buffer_size(&mut self, buffer_size: u32) -> Result<()> {
        if !buffer_size.is_power_of_two() {
            return Err(anyhow!("Buffer size must be power of 2"));
        }
        if buffer_size < 64 || buffer_size > 4096 {
            return Err(anyhow!("Buffer size must be between 64 and 4096"));
        }
        // Valid buffer size - would need to reinitialize streams
        Ok(())
    }
}

impl Drop for RealTimeAudioProcessor {
    fn drop(&mut self) {
        if self.is_running.load(Ordering::Relaxed) {
            let _ = self.stop();
        }
    }
}

/// Audio buffer pool for efficient memory management
pub struct AudioBufferPool {
    capacity: usize,
    available: Arc<Mutex<Vec<Vec<f32>>>>,
}

impl AudioBufferPool {
    /// Create new buffer pool with given capacity
    pub fn new(capacity: usize) -> Self {
        let mut buffers = Vec::new();
        for _ in 0..capacity {
            buffers.push(vec![0.0; FRAME_SIZE_SAMPLES * CHANNELS as usize]);
        }

        Self {
            capacity,
            available: Arc::new(Mutex::new(buffers)),
        }
    }

    /// Get number of total buffers in pool
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Get number of available buffers
    pub fn available(&self) -> usize {
        self.available.lock().unwrap().len()
    }

    /// Get number of allocated buffers
    pub fn allocated(&self) -> usize {
        self.capacity - self.available()
    }

    /// Acquire a buffer from the pool
    pub fn acquire(&self) -> Option<Vec<f32>> {
        self.available.lock().unwrap().pop()
    }

    /// Clear all buffers (force release)
    pub fn clear(&self) {
        let mut available = self.available.lock().unwrap();
        available.clear();
        for _ in 0..self.capacity {
            available.push(vec![0.0; FRAME_SIZE_SAMPLES * CHANNELS as usize]);
        }
    }
}

/// Set real-time thread priority for low-latency audio processing
pub fn set_realtime_priority() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        use libc::{sched_setscheduler, sched_param, SCHED_FIFO};
        let param = sched_param { sched_priority: 80 };
        let result = unsafe { sched_setscheduler(0, SCHED_FIFO, &param) };
        if result != 0 {
            return Err(anyhow!("Failed to set real-time priority"));
        }
    }

    #[cfg(target_os = "macos")]
    {
        // macOS uses different thread priority APIs
        warn!("Real-time priority setting not implemented for macOS");
    }

    #[cfg(target_os = "windows")]
    {
        // Windows uses different thread priority APIs
        warn!("Real-time priority setting not implemented for Windows");
    }

    Ok(())
}

/// Audio processing statistics
#[derive(Debug, Clone)]
pub struct AudioStats {
    pub frames_processed: u64,
    pub last_input_time: u64,
    pub last_output_time: u64,
    pub input_buffer_usage: usize,
    pub output_buffer_usage: usize,
    pub is_running: bool,
    pub input_underruns: u64,
    pub output_overruns: u64,
}

impl AudioStats {
    pub fn input_latency_ms(&self) -> f64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        (now.saturating_sub(self.last_input_time)) as f64
    }

    pub fn output_latency_ms(&self) -> f64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        (now.saturating_sub(self.last_output_time)) as f64
    }
}