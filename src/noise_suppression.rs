use anyhow::Result;
use log::info;
use std::collections::VecDeque;
use crate::realtime_audio::{AudioFrame, SAMPLE_RATE, CHANNELS, FRAME_SIZE_SAMPLES};

/// Noise suppression configuration
#[derive(Debug, Clone)]
pub struct NoiseSuppressionConfig {
    /// Noise reduction strength (0.0 = off, 1.0 = maximum)
    pub strength: f32,
    /// Minimum noise floor in dB
    pub noise_floor_db: f32,
    /// Attack time for noise gate in milliseconds
    pub attack_time_ms: f32,
    /// Release time for noise gate in milliseconds
    pub release_time_ms: f32,
    /// Spectral subtraction factor
    pub spectral_subtraction_factor: f32,
    /// Enable adaptive mode
    pub adaptive: bool,
}

impl Default for NoiseSuppressionConfig {
    fn default() -> Self {
        Self {
            strength: 0.7,                    // 70% noise reduction
            noise_floor_db: -50.0,            // -50dB noise floor
            attack_time_ms: 5.0,              // 5ms attack
            release_time_ms: 50.0,            // 50ms release
            spectral_subtraction_factor: 2.0, // Conservative spectral subtraction
            adaptive: true,                   // Enable adaptive noise tracking
        }
    }
}

/// Real-time noise suppression processor
pub struct NoiseSuppressionProcessor {
    config: NoiseSuppressionConfig,

    // Noise estimation
    noise_estimate: Vec<f32>,
    noise_update_rate: f32,
    signal_energy_history: VecDeque<f32>,

    // Spectral processing buffers
    fft_size: usize,
    overlap_size: usize,
    window: Vec<f32>,
    input_buffer: VecDeque<f32>,
    output_buffer: VecDeque<f32>,

    // FFT workspace (simple DFT for now)
    spectrum_real: Vec<f32>,
    spectrum_imag: Vec<f32>,
    magnitude_spectrum: Vec<f32>,
    phase_spectrum: Vec<f32>,

    // Noise gate
    envelope_follower: f32,
    gate_state: GateState,

    // Statistics
    frames_processed: u64,
    noise_reduction_applied: f32,
}

#[derive(Debug, Clone, PartialEq)]
enum GateState {
    Open,
    Closed,
    Attack,
    Release,
}

impl NoiseSuppressionProcessor {
    /// Create new noise suppression processor
    pub fn new(config: NoiseSuppressionConfig) -> Result<Self> {
        info!("Creating noise suppression processor with strength: {:.1}%",
              config.strength * 100.0);

        // Use 512-point FFT for reasonable frequency resolution
        let fft_size = 512;
        let overlap_size = fft_size / 2; // 50% overlap

        // Create Hann window for smooth spectral analysis
        let mut window = vec![0.0; fft_size];
        for i in 0..fft_size {
            window[i] = 0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / (fft_size - 1) as f32).cos());
        }

        // Initialize noise estimate to small values
        let noise_estimate = vec![1e-6; fft_size / 2 + 1];

        Ok(Self {
            config,
            noise_estimate,
            noise_update_rate: 0.01, // 1% per frame for noise adaptation
            signal_energy_history: VecDeque::with_capacity(100),
            fft_size,
            overlap_size,
            window,
            input_buffer: VecDeque::with_capacity(fft_size * 2),
            output_buffer: VecDeque::with_capacity(fft_size * 2),
            spectrum_real: vec![0.0; fft_size],
            spectrum_imag: vec![0.0; fft_size],
            magnitude_spectrum: vec![0.0; fft_size / 2 + 1],
            phase_spectrum: vec![0.0; fft_size / 2 + 1],
            envelope_follower: 0.0,
            gate_state: GateState::Closed,
            frames_processed: 0,
            noise_reduction_applied: 0.0,
        })
    }

    /// Process audio frame with noise suppression
    pub fn process_frame(&mut self, frame: &mut AudioFrame) -> Result<()> {
        self.frames_processed += 1;

        // Process each channel separately
        let samples_per_channel = FRAME_SIZE_SAMPLES;

        for channel in 0..CHANNELS as usize {
            // Extract channel samples
            let mut channel_samples = Vec::with_capacity(samples_per_channel);
            for i in 0..samples_per_channel {
                let sample_idx = i * CHANNELS as usize + channel;
                if sample_idx < frame.samples.len() {
                    channel_samples.push(frame.samples[sample_idx]);
                }
            }

            // Apply noise suppression to channel
            self.process_channel(&mut channel_samples)?;

            // Write back processed samples
            for i in 0..samples_per_channel {
                let sample_idx = i * CHANNELS as usize + channel;
                if sample_idx < frame.samples.len() && i < channel_samples.len() {
                    frame.samples[sample_idx] = channel_samples[i];
                }
            }
        }

        Ok(())
    }

    /// Process single channel with noise suppression
    fn process_channel(&mut self, samples: &mut [f32]) -> Result<()> {
        // Add samples to input buffer
        for &sample in samples.iter() {
            self.input_buffer.push_back(sample);
        }

        // Process in overlapping frames
        let mut output_samples: Vec<f32> = Vec::new();

        while self.input_buffer.len() >= self.fft_size {
            // Extract frame for processing
            let mut frame_samples: Vec<f32> = self.input_buffer.iter()
                .take(self.fft_size)
                .copied()
                .collect();

            // Apply windowing
            for i in 0..self.fft_size {
                frame_samples[i] *= self.window[i];
            }

            // Simple spectral processing
            self.spectral_noise_suppression(&mut frame_samples)?;

            // Overlap-add output
            for (i, &sample) in frame_samples.iter().enumerate() {
                if i < self.overlap_size {
                    // Overlap region - add to existing samples
                    if let Some(existing) = self.output_buffer.get_mut(i) {
                        *existing += sample;
                    } else {
                        self.output_buffer.push_back(sample);
                    }
                } else {
                    // New region
                    self.output_buffer.push_back(sample);
                }
            }

            // Remove processed samples from input (hop size)
            for _ in 0..self.overlap_size {
                self.input_buffer.pop_front();
            }
        }

        // Extract output samples
        let samples_needed = samples.len();
        for i in 0..samples_needed {
            if let Some(sample) = self.output_buffer.pop_front() {
                samples[i] = sample;
            }
        }

        Ok(())
    }

    /// Apply spectral noise suppression
    fn spectral_noise_suppression(&mut self, frame: &mut [f32]) -> Result<()> {
        // Simple DFT (in production, would use optimized FFT)
        self.compute_dft(frame);

        // Update noise estimate
        if self.config.adaptive {
            self.update_noise_estimate();
        }

        // Apply spectral subtraction
        self.apply_spectral_subtraction();

        // Convert back to time domain
        self.compute_inverse_dft(frame);

        // Apply noise gate
        self.apply_noise_gate(frame);

        Ok(())
    }

    /// Compute simple DFT (placeholder for optimized FFT)
    fn compute_dft(&mut self, frame: &[f32]) {
        let n = self.fft_size;

        // Clear spectrum buffers
        self.spectrum_real.fill(0.0);
        self.spectrum_imag.fill(0.0);

        // Compute DFT
        for k in 0..n {
            for i in 0..n {
                let angle = -2.0 * std::f32::consts::PI * (k * i) as f32 / n as f32;
                self.spectrum_real[k] += frame[i] * angle.cos();
                self.spectrum_imag[k] += frame[i] * angle.sin();
            }
        }

        // Compute magnitude and phase for positive frequencies
        for k in 0..=n/2 {
            self.magnitude_spectrum[k] = (self.spectrum_real[k].powi(2) + self.spectrum_imag[k].powi(2)).sqrt();
            self.phase_spectrum[k] = self.spectrum_imag[k].atan2(self.spectrum_real[k]);
        }
    }

    /// Update noise estimate using adaptive algorithm
    fn update_noise_estimate(&mut self) {
        // Calculate current frame energy
        let frame_energy: f32 = self.magnitude_spectrum.iter()
            .map(|&mag| mag.powi(2))
            .sum();

        self.signal_energy_history.push_back(frame_energy);
        if self.signal_energy_history.len() > 100 {
            self.signal_energy_history.pop_front();
        }

        // Determine if current frame is likely noise (low energy, stable)
        let is_noise_frame = if self.signal_energy_history.len() > 10 {
            let recent_avg: f32 = self.signal_energy_history.iter()
                .rev()
                .take(10)
                .sum::<f32>() / 10.0;

            frame_energy < recent_avg * 1.5 // Current frame is not much louder than recent average
        } else {
            true // Assume noise during startup
        };

        // Update noise estimate for each frequency bin
        if is_noise_frame {
            for k in 0..self.noise_estimate.len() {
                let current_mag = if k < self.magnitude_spectrum.len() {
                    self.magnitude_spectrum[k]
                } else {
                    0.0
                };

                // Exponential moving average
                self.noise_estimate[k] = (1.0 - self.noise_update_rate) * self.noise_estimate[k]
                                       + self.noise_update_rate * current_mag;
            }
        }
    }

    /// Apply spectral subtraction for noise reduction
    fn apply_spectral_subtraction(&mut self) {
        for k in 0..self.magnitude_spectrum.len() {
            let signal_mag = self.magnitude_spectrum[k];
            let noise_mag = self.noise_estimate[k];

            // Spectral subtraction with over-subtraction factor
            let enhanced_mag = signal_mag - self.config.spectral_subtraction_factor * noise_mag;

            // Apply noise floor to prevent over-subtraction artifacts
            let noise_floor = noise_mag * 0.1; // 10% of noise level as floor
            let final_mag = enhanced_mag.max(noise_floor);

            // Calculate suppression factor
            let suppression = if signal_mag > 0.0 {
                (final_mag / signal_mag).min(1.0)
            } else {
                0.0
            };

            // Apply suppression with strength control
            let effective_suppression = 1.0 - self.config.strength * (1.0 - suppression);
            self.magnitude_spectrum[k] = signal_mag * effective_suppression;

            // Update statistics
            self.noise_reduction_applied = self.config.strength * (1.0 - suppression);
        }
    }

    /// Convert back to time domain (inverse DFT)
    fn compute_inverse_dft(&mut self, frame: &mut [f32]) {
        let n = self.fft_size;

        // Reconstruct complex spectrum from magnitude and phase
        for k in 0..=n/2 {
            let mag = self.magnitude_spectrum[k];
            let phase = self.phase_spectrum[k];

            self.spectrum_real[k] = mag * phase.cos();
            self.spectrum_imag[k] = mag * phase.sin();

            // Mirror for negative frequencies (except DC and Nyquist)
            if k > 0 && k < n/2 {
                self.spectrum_real[n - k] = self.spectrum_real[k];
                self.spectrum_imag[n - k] = -self.spectrum_imag[k];
            }
        }

        // Compute inverse DFT
        for i in 0..n {
            frame[i] = 0.0;
            for k in 0..n {
                let angle = 2.0 * std::f32::consts::PI * (k * i) as f32 / n as f32;
                frame[i] += self.spectrum_real[k] * angle.cos() - self.spectrum_imag[k] * angle.sin();
            }
            frame[i] /= n as f32; // Normalize
        }
    }

    /// Apply noise gate for additional noise suppression
    fn apply_noise_gate(&mut self, frame: &mut [f32]) {
        // Calculate frame RMS
        let rms: f32 = (frame.iter().map(|&x| x.powi(2)).sum::<f32>() / frame.len() as f32).sqrt();

        // Convert to dB
        let _rms_db = if rms > 0.0 {
            20.0 * rms.log10()
        } else {
            -80.0 // Very quiet
        };

        // Update envelope follower
        let attack_coeff = (-1.0 / (self.config.attack_time_ms * SAMPLE_RATE as f32 / 1000.0)).exp();
        let release_coeff = (-1.0 / (self.config.release_time_ms * SAMPLE_RATE as f32 / 1000.0)).exp();

        if rms > self.envelope_follower {
            // Attack
            self.envelope_follower = attack_coeff * self.envelope_follower + (1.0 - attack_coeff) * rms;
        } else {
            // Release
            self.envelope_follower = release_coeff * self.envelope_follower + (1.0 - release_coeff) * rms;
        }

        // Determine gate state
        let envelope_db = if self.envelope_follower > 0.0 {
            20.0 * self.envelope_follower.log10()
        } else {
            -80.0
        };

        let gate_threshold = self.config.noise_floor_db + 6.0; // 6dB above noise floor

        let new_state = if envelope_db > gate_threshold {
            match self.gate_state {
                GateState::Closed | GateState::Release => GateState::Attack,
                _ => GateState::Open,
            }
        } else {
            match self.gate_state {
                GateState::Open | GateState::Attack => GateState::Release,
                _ => GateState::Closed,
            }
        };

        // Apply gate
        let gate_gain = match new_state {
            GateState::Open => 1.0,
            GateState::Closed => 0.1, // -20dB attenuation
            GateState::Attack | GateState::Release => {
                // Smooth transition
                let progress = (envelope_db - self.config.noise_floor_db) / 6.0;
                0.1 + 0.9 * progress.clamp(0.0, 1.0)
            }
        };

        self.gate_state = new_state;

        // Apply gate gain
        for sample in frame.iter_mut() {
            *sample *= gate_gain;
        }
    }

    /// Update noise suppression strength
    pub fn set_strength(&mut self, strength: f32) {
        self.config.strength = strength.clamp(0.0, 1.0);
        info!("Noise suppression strength updated to {:.1}%", self.config.strength * 100.0);
    }

    /// Get current configuration
    pub fn get_config(&self) -> &NoiseSuppressionConfig {
        &self.config
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> NoiseSuppressionStats {
        NoiseSuppressionStats {
            frames_processed: self.frames_processed,
            current_strength: self.config.strength,
            noise_reduction_applied: self.noise_reduction_applied,
            gate_state: self.gate_state.clone(),
            noise_floor_estimate: self.noise_estimate.iter().sum::<f32>() / self.noise_estimate.len() as f32,
        }
    }

    /// Reset processor state
    pub fn reset(&mut self) {
        info!("Resetting noise suppression processor");

        self.input_buffer.clear();
        self.output_buffer.clear();
        self.signal_energy_history.clear();
        self.envelope_follower = 0.0;
        self.gate_state = GateState::Closed;
        self.frames_processed = 0;
        self.noise_reduction_applied = 0.0;

        // Reset noise estimate
        self.noise_estimate.fill(1e-6);
    }
}

/// Noise suppression statistics
#[derive(Debug, Clone)]
pub struct NoiseSuppressionStats {
    pub frames_processed: u64,
    pub current_strength: f32,
    pub noise_reduction_applied: f32,
    pub gate_state: GateState,
    pub noise_floor_estimate: f32,
}

impl NoiseSuppressionStats {
    pub fn noise_reduction_db(&self) -> f32 {
        if self.noise_reduction_applied > 0.0 {
            -20.0 * self.noise_reduction_applied.log10()
        } else {
            0.0
        }
    }
}