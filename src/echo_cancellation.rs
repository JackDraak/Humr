use anyhow::Result;
use log::info;
use std::collections::VecDeque;
use crate::realtime_audio::{AudioFrame, SAMPLE_RATE, CHANNELS, FRAME_SIZE_SAMPLES};

/// Echo cancellation configuration
#[derive(Debug, Clone)]
pub struct EchoCancellationConfig {
    /// Maximum echo delay in milliseconds
    pub max_echo_delay_ms: f32,
    /// Echo suppression strength (0.0 = off, 1.0 = maximum)
    pub suppression_strength: f32,
    /// Adaptive learning rate
    pub learning_rate: f32,
    /// Echo threshold for detection
    pub echo_threshold: f32,
    /// Enable non-linear processing
    pub nonlinear_processing: bool,
    /// Filter length for adaptive filter
    pub filter_length: usize,
}

impl Default for EchoCancellationConfig {
    fn default() -> Self {
        Self {
            max_echo_delay_ms: 200.0,      // 200ms max echo delay
            suppression_strength: 0.8,      // 80% echo suppression
            learning_rate: 0.01,            // 1% learning rate
            echo_threshold: 0.01,           // Echo detection threshold
            nonlinear_processing: true,     // Enable nonlinear processing
            filter_length: 512,             // 512-tap adaptive filter
        }
    }
}

/// Acoustic Echo Cancellation (AEC) processor
pub struct EchoCancellationProcessor {
    config: EchoCancellationConfig,

    // Adaptive filter coefficients
    filter_coeffs: Vec<f32>,

    // Signal buffers
    reference_buffer: VecDeque<f32>,    // Far-end (speaker) signal
    microphone_buffer: VecDeque<f32>,   // Near-end (microphone) signal
    error_buffer: VecDeque<f32>,        // Error signal for adaptation

    // Echo estimation
    estimated_echo: Vec<f32>,
    echo_suppression_gain: f32,

    // Statistics and monitoring
    frames_processed: u64,
    echo_detected: bool,
    echo_suppression_db: f32,
    adaptation_active: bool,

    // Double-talk detection
    far_end_power: f32,
    near_end_power: f32,
    echo_power: f32,
    double_talk_threshold: f32,

    // Nonlinear processor state
    nonlinear_suppression: f32,
    comfort_noise_level: f32,
}

impl EchoCancellationProcessor {
    /// Create new echo cancellation processor
    pub fn new(config: EchoCancellationConfig) -> Result<Self> {
        info!("Creating echo cancellation processor with {}ms max delay",
              config.max_echo_delay_ms);

        // Calculate buffer sizes
        let max_delay_samples = (config.max_echo_delay_ms * SAMPLE_RATE as f32 / 1000.0) as usize;
        let buffer_capacity = max_delay_samples + config.filter_length;

        // Initialize adaptive filter coefficients
        let filter_coeffs = vec![0.0; config.filter_length];

        Ok(Self {
            config,
            filter_coeffs,
            reference_buffer: VecDeque::with_capacity(buffer_capacity),
            microphone_buffer: VecDeque::with_capacity(buffer_capacity),
            error_buffer: VecDeque::with_capacity(1000), // Store recent errors
            estimated_echo: vec![0.0; FRAME_SIZE_SAMPLES],
            echo_suppression_gain: 1.0,
            frames_processed: 0,
            echo_detected: false,
            echo_suppression_db: 0.0,
            adaptation_active: false,
            far_end_power: 0.0,
            near_end_power: 0.0,
            echo_power: 0.0,
            double_talk_threshold: 0.1,
            nonlinear_suppression: 1.0,
            comfort_noise_level: 0.001, // -60dB comfort noise
        })
    }

    /// Process audio frame with echo cancellation
    /// reference_frame: Far-end signal (what the speaker will play)
    /// microphone_frame: Near-end signal (what the microphone captures)
    /// Returns: Echo-cancelled microphone signal
    pub fn process_frame(&mut self, reference_frame: &AudioFrame, microphone_frame: &mut AudioFrame) -> Result<()> {
        self.frames_processed += 1;

        // Process each channel separately
        let samples_per_channel = FRAME_SIZE_SAMPLES;

        for channel in 0..CHANNELS as usize {
            // Extract channel samples
            let mut ref_samples = Vec::with_capacity(samples_per_channel);
            let mut mic_samples = Vec::with_capacity(samples_per_channel);

            for i in 0..samples_per_channel {
                let sample_idx = i * CHANNELS as usize + channel;

                if sample_idx < reference_frame.samples.len() {
                    ref_samples.push(reference_frame.samples[sample_idx]);
                }

                if sample_idx < microphone_frame.samples.len() {
                    mic_samples.push(microphone_frame.samples[sample_idx]);
                }
            }

            // Apply echo cancellation to channel
            self.process_channel(&ref_samples, &mut mic_samples)?;

            // Write back processed microphone samples
            for i in 0..samples_per_channel {
                let sample_idx = i * CHANNELS as usize + channel;
                if sample_idx < microphone_frame.samples.len() && i < mic_samples.len() {
                    microphone_frame.samples[sample_idx] = mic_samples[i];
                }
            }
        }

        Ok(())
    }

    /// Process single channel with echo cancellation
    fn process_channel(&mut self, reference: &[f32], microphone: &mut [f32]) -> Result<()> {
        // Add samples to buffers
        for &sample in reference {
            self.reference_buffer.push_back(sample);
        }

        for sample in microphone.iter() {
            self.microphone_buffer.push_back(*sample);
        }

        // Maintain buffer sizes
        let max_buffer_size = self.config.filter_length * 4;
        while self.reference_buffer.len() > max_buffer_size {
            self.reference_buffer.pop_front();
        }
        while self.microphone_buffer.len() > max_buffer_size {
            self.microphone_buffer.pop_front();
        }

        // Calculate signal powers for double-talk detection
        self.update_signal_powers(reference, microphone);

        // Detect double-talk (simultaneous near-end and far-end speech)
        let double_talk_detected = self.detect_double_talk();

        // Apply adaptive filtering if not in double-talk
        if !double_talk_detected {
            self.adaptive_filter(reference, microphone)?;
        }

        // Apply echo suppression
        self.apply_echo_suppression(microphone);

        // Apply nonlinear processing if enabled
        if self.config.nonlinear_processing {
            self.apply_nonlinear_processing(microphone);
        }

        Ok(())
    }

    /// Update signal power estimates for double-talk detection
    fn update_signal_powers(&mut self, reference: &[f32], microphone: &[f32]) {
        // Calculate far-end power
        let ref_power: f32 = reference.iter().map(|&x| x.powi(2)).sum::<f32>() / reference.len() as f32;
        self.far_end_power = 0.9 * self.far_end_power + 0.1 * ref_power;

        // Calculate near-end power
        let mic_power: f32 = microphone.iter().map(|&x| x.powi(2)).sum::<f32>() / microphone.len() as f32;
        self.near_end_power = 0.9 * self.near_end_power + 0.1 * mic_power;

        // Calculate estimated echo power
        let echo_power: f32 = self.estimated_echo.iter().map(|&x| x.powi(2)).sum::<f32>() / self.estimated_echo.len() as f32;
        self.echo_power = 0.9 * self.echo_power + 0.1 * echo_power;
    }

    /// Detect if double-talk is occurring (simultaneous near and far-end speech)
    fn detect_double_talk(&self) -> bool {
        // Simple double-talk detector based on power ratios
        let echo_return_loss = if self.far_end_power > 1e-10 {
            10.0 * (self.near_end_power / self.far_end_power).log10()
        } else {
            0.0
        };

        // If near-end power is significantly higher than expected echo,
        // we likely have double-talk
        echo_return_loss > self.double_talk_threshold
    }

    /// Apply adaptive filtering using LMS algorithm
    fn adaptive_filter(&mut self, _reference: &[f32], microphone: &[f32]) -> Result<()> {
        if self.reference_buffer.len() < self.config.filter_length {
            return Ok(()); // Not enough reference data yet
        }

        self.adaptation_active = true;

        // Process each sample
        for (i, &mic_sample) in microphone.iter().enumerate() {
            // Get reference samples for filter
            let ref_start = self.reference_buffer.len().saturating_sub(self.config.filter_length + i);
            let ref_slice: Vec<f32> = self.reference_buffer.iter()
                .skip(ref_start)
                .take(self.config.filter_length)
                .copied()
                .collect();

            if ref_slice.len() != self.config.filter_length {
                continue;
            }

            // Estimate echo using current filter
            let estimated_echo: f32 = self.filter_coeffs.iter()
                .zip(ref_slice.iter())
                .map(|(&coeff, &ref_val)| coeff * ref_val)
                .sum();

            // Calculate error (desired signal)
            let error = mic_sample - estimated_echo;

            // Store estimated echo for this sample
            if i < self.estimated_echo.len() {
                self.estimated_echo[i] = estimated_echo;
            }

            // Update filter coefficients using LMS algorithm
            let step_size = self.config.learning_rate / (1e-6 + ref_slice.iter().map(|&x| x.powi(2)).sum::<f32>());

            for (coeff, &ref_val) in self.filter_coeffs.iter_mut().zip(ref_slice.iter()) {
                *coeff += step_size * error * ref_val;

                // Prevent coefficient explosion
                *coeff = coeff.clamp(-1.0, 1.0);
            }

            // Store error for monitoring
            self.error_buffer.push_back(error);
            if self.error_buffer.len() > 1000 {
                self.error_buffer.pop_front();
            }
        }

        Ok(())
    }

    /// Apply echo suppression based on estimated echo
    fn apply_echo_suppression(&mut self, microphone: &mut [f32]) {
        // Calculate suppression gain based on echo estimate
        let mic_power: f32 = microphone.iter().map(|&x| x.powi(2)).sum::<f32>() / microphone.len() as f32;
        let echo_power: f32 = self.estimated_echo.iter().map(|&x| x.powi(2)).sum::<f32>() / self.estimated_echo.len() as f32;

        // Determine if echo is present
        self.echo_detected = echo_power > self.config.echo_threshold * mic_power;

        if self.echo_detected {
            // Calculate suppression factor
            let echo_ratio = if mic_power > 1e-10 {
                echo_power / mic_power
            } else {
                0.0
            };

            // Apply suppression with strength control
            let suppression_factor = 1.0 - self.config.suppression_strength * echo_ratio.min(1.0);
            self.echo_suppression_gain = suppression_factor.max(0.1); // Minimum 10% gain

            // Calculate suppression in dB for monitoring
            self.echo_suppression_db = -20.0 * (self.echo_suppression_gain).log10();

            // Apply suppression
            for sample in microphone.iter_mut() {
                *sample *= self.echo_suppression_gain;
            }
        } else {
            self.echo_suppression_gain = 1.0;
            self.echo_suppression_db = 0.0;
        }
    }

    /// Apply nonlinear processing for residual echo suppression
    fn apply_nonlinear_processing(&mut self, microphone: &mut [f32]) {
        if !self.echo_detected {
            self.nonlinear_suppression = 1.0;
            return;
        }

        // Calculate residual echo estimate
        let residual_power: f32 = microphone.iter().map(|&x| x.powi(2)).sum::<f32>() / microphone.len() as f32;

        // Determine nonlinear suppression factor
        let target_suppression = if residual_power > self.config.echo_threshold {
            0.3 // Strong suppression for residual echo
        } else {
            0.8 // Light suppression
        };

        // Smooth suppression changes
        self.nonlinear_suppression = 0.9 * self.nonlinear_suppression + 0.1 * target_suppression;

        // Apply nonlinear suppression with comfort noise
        for sample in microphone.iter_mut() {
            // Apply suppression
            *sample *= self.nonlinear_suppression;

            // Add comfort noise to mask artifacts
            let noise = (rand::random::<f32>() - 0.5) * self.comfort_noise_level;
            *sample += noise;
        }
    }

    /// Update echo cancellation configuration
    pub fn update_config(&mut self, config: EchoCancellationConfig) {
        info!("Updating echo cancellation config");

        // If filter length changed, resize filter
        if config.filter_length != self.config.filter_length {
            self.filter_coeffs.resize(config.filter_length, 0.0);
        }

        self.config = config;
    }

    /// Get current configuration
    pub fn get_config(&self) -> &EchoCancellationConfig {
        &self.config
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> EchoCancellationStats {
        EchoCancellationStats {
            frames_processed: self.frames_processed,
            echo_detected: self.echo_detected,
            echo_suppression_db: self.echo_suppression_db,
            adaptation_active: self.adaptation_active,
            far_end_power: self.far_end_power,
            near_end_power: self.near_end_power,
            echo_power: self.echo_power,
            filter_convergence: self.calculate_filter_convergence(),
        }
    }

    /// Calculate filter convergence metric
    fn calculate_filter_convergence(&self) -> f32 {
        if self.error_buffer.len() < 100 {
            return 0.0;
        }

        // Calculate recent error variance as convergence metric
        let recent_errors: Vec<f32> = self.error_buffer.iter().rev().take(100).copied().collect();
        let mean_error: f32 = recent_errors.iter().sum::<f32>() / recent_errors.len() as f32;
        let error_variance: f32 = recent_errors.iter()
            .map(|&e| (e - mean_error).powi(2))
            .sum::<f32>() / recent_errors.len() as f32;

        // Return convergence as 1 - normalized_variance
        (1.0 - (error_variance * 1000.0).min(1.0)).max(0.0)
    }

    /// Add audio frame for processing (convenience method)
    pub fn add_frame(&mut self, frame: &crate::realtime_audio::AudioFrame) {
        // This is a convenience method that could be used for buffering frames
        // For now, it's a no-op since the main processing happens in process_frame
        self.frames_processed += 1;
    }

    /// Reset processor state
    pub fn reset(&mut self) {
        info!("Resetting echo cancellation processor");

        self.filter_coeffs.fill(0.0);
        self.reference_buffer.clear();
        self.microphone_buffer.clear();
        self.error_buffer.clear();
        self.estimated_echo.fill(0.0);

        self.echo_suppression_gain = 1.0;
        self.frames_processed = 0;
        self.echo_detected = false;
        self.echo_suppression_db = 0.0;
        self.adaptation_active = false;

        self.far_end_power = 0.0;
        self.near_end_power = 0.0;
        self.echo_power = 0.0;
        self.nonlinear_suppression = 1.0;
    }
}

/// Echo cancellation statistics
#[derive(Debug, Clone)]
pub struct EchoCancellationStats {
    pub frames_processed: u64,
    pub echo_detected: bool,
    pub echo_suppression_db: f32,
    pub adaptation_active: bool,
    pub far_end_power: f32,
    pub near_end_power: f32,
    pub echo_power: f32,
    pub filter_convergence: f32,
}

impl EchoCancellationStats {
    /// Get echo return loss in dB
    pub fn echo_return_loss_db(&self) -> f32 {
        if self.far_end_power > 1e-10 {
            10.0 * (self.near_end_power / self.far_end_power).log10()
        } else {
            0.0
        }
    }

    /// Check if the filter has converged sufficiently
    pub fn is_converged(&self) -> bool {
        self.filter_convergence > 0.8
    }
}