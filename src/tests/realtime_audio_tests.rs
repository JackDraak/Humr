#[cfg(test)]
mod realtime_audio_tests {
    use crate::realtime_audio::*;
    use crate::realtime_audio::AudioFrame;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Duration;

    #[test]
    fn test_audio_frame_creation() {
        let samples = vec![0.5, -0.3, 0.8, -0.1];
        let frame = AudioFrame::new(samples.clone());

        assert_eq!(frame.samples, samples);
        assert_eq!(frame.samples.len(), 4);
    }

    #[test]
    fn test_audio_frame_constants() {
        assert_eq!(SAMPLE_RATE, 48000);
        assert_eq!(CHANNELS, 2);
        assert_eq!(FRAME_SIZE_MS, 20);
        assert_eq!(FRAME_SIZE_SAMPLES, 1920); // 48000 * 0.02
    }

    #[test]
    fn test_audio_frame_with_frame_size() {
        let frame = AudioFrame::new(vec![0.0; FRAME_SIZE_SAMPLES]);
        assert_eq!(frame.samples.len(), FRAME_SIZE_SAMPLES);
    }

    #[test]
    fn test_audio_frame_normalization() {
        let mut frame = AudioFrame::new(vec![2.0, -3.0, 0.5, 1.5]);
        frame.normalize();

        // Check all samples are within [-1.0, 1.0] range
        for sample in &frame.samples {
            assert!(*sample >= -1.0 && *sample <= 1.0);
        }

        // Check that the maximum absolute value is 1.0 (or close to it)
        let max_abs = frame.samples.iter().map(|s| s.abs()).fold(0.0, f32::max);
        assert!((max_abs - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_audio_frame_apply_gain() {
        let mut frame = AudioFrame::new(vec![0.5, -0.5, 0.25, -0.25]);
        frame.apply_gain(2.0);

        assert_eq!(frame.samples, vec![1.0, -1.0, 0.5, -0.5]);
    }

    #[test]
    fn test_audio_frame_mix() {
        let mut frame1 = AudioFrame::new(vec![0.5, 0.3, 0.1]);
        let frame2 = AudioFrame::new(vec![0.2, 0.4, 0.6]);

        frame1.mix(&frame2, 0.5);

        // frame1 should now contain 0.5 * frame1 + 0.5 * frame2
        let expected = vec![0.35, 0.35, 0.35]; // (0.5*0.5 + 0.5*0.2, etc.)
        for (actual, expected) in frame1.samples.iter().zip(expected.iter()) {
            assert!((actual - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn test_audio_frame_is_silent() {
        let silent_frame = AudioFrame::new(vec![0.0, 0.0, 0.0, 0.0]);
        assert!(silent_frame.is_silent(0.01));

        let quiet_frame = AudioFrame::new(vec![0.005, -0.003, 0.001, 0.0]);
        assert!(quiet_frame.is_silent(0.01));

        let loud_frame = AudioFrame::new(vec![0.5, 0.0, 0.0, 0.0]);
        assert!(!loud_frame.is_silent(0.01));
    }

    #[test]
    fn test_audio_frame_rms() {
        let frame = AudioFrame::new(vec![1.0, 0.0, -1.0, 0.0]);
        let rms = frame.rms();

        // RMS of [1, 0, -1, 0] = sqrt((1 + 0 + 1 + 0) / 4) = sqrt(0.5)
        let expected_rms = (0.5_f32).sqrt();
        assert!((rms - expected_rms).abs() < 1e-6);
    }

    #[test]
    fn test_audio_frame_peak() {
        let frame = AudioFrame::new(vec![0.5, -0.8, 0.3, -0.2]);
        assert_eq!(frame.peak(), 0.8);

        let silent_frame = AudioFrame::new(vec![0.0, 0.0, 0.0]);
        assert_eq!(silent_frame.peak(), 0.0);
    }

    #[test]
    fn test_realtime_audio_processor_creation() {
        let processor = RealTimeAudioProcessor::new();
        assert!(processor.is_ok());

        let processor = processor.unwrap();
        assert!(!processor.is_running());
        assert_eq!(processor.get_stats().frames_processed, 0);
    }

    #[test]
    fn test_audio_processor_stats() {
        let processor = RealTimeAudioProcessor::new().unwrap();
        let stats = processor.get_stats();

        assert_eq!(stats.frames_processed, 0);
        assert!(stats.input_latency_ms() >= 0.0);
        assert!(stats.output_latency_ms() >= 0.0);
        assert_eq!(stats.input_underruns, 0);
        assert_eq!(stats.output_overruns, 0);
    }

    #[test]
    fn test_audio_buffer_pool_creation() {
        let pool = AudioBufferPool::new(10);
        assert_eq!(pool.capacity(), 10);
        assert_eq!(pool.available(), 10);
        assert_eq!(pool.allocated(), 0);
    }

    #[test]
    fn test_audio_buffer_pool_acquire_release() {
        let pool = AudioBufferPool::new(2);

        // Acquire first buffer
        let buffer1 = pool.acquire();
        assert!(buffer1.is_some());
        assert_eq!(pool.available(), 1);
        assert_eq!(pool.allocated(), 1);

        // Acquire second buffer
        let buffer2 = pool.acquire();
        assert!(buffer2.is_some());
        assert_eq!(pool.available(), 0);
        assert_eq!(pool.allocated(), 2);

        // Try to acquire third buffer (should fail)
        let buffer3 = pool.acquire();
        assert!(buffer3.is_none());

        // Release first buffer
        drop(buffer1);
        assert_eq!(pool.available(), 1);
        assert_eq!(pool.allocated(), 1);

        // Now we should be able to acquire again
        let buffer4 = pool.acquire();
        assert!(buffer4.is_some());
    }

    #[test]
    fn test_audio_buffer_pool_clear() {
        let pool = AudioBufferPool::new(5);
        let _buffer1 = pool.acquire();
        let _buffer2 = pool.acquire();

        assert_eq!(pool.available(), 3);
        assert_eq!(pool.allocated(), 2);

        pool.clear();
        assert_eq!(pool.available(), 5);
        assert_eq!(pool.allocated(), 0);
    }

    #[test]
    fn test_audio_processor_config() {
        let mut processor = RealTimeAudioProcessor::new().unwrap();

        // Test setting sample rate
        assert!(processor.set_sample_rate(44100).is_ok());
        assert!(processor.set_sample_rate(96000).is_ok());

        // Test invalid sample rate
        assert!(processor.set_sample_rate(1000).is_err());

        // Test setting buffer size
        assert!(processor.set_buffer_size(256).is_ok());
        assert!(processor.set_buffer_size(1024).is_ok());

        // Test invalid buffer size
        assert!(processor.set_buffer_size(7).is_err()); // Not power of 2
        assert!(processor.set_buffer_size(8192).is_err()); // Too large
    }

    #[test]
    fn test_thread_priority_setting() {
        let result = set_realtime_priority();
        // This may fail on systems without proper permissions
        // but should not panic
        match result {
            Ok(_) => println!("Real-time priority set successfully"),
            Err(e) => println!("Failed to set real-time priority (expected on some systems): {}", e),
        }
    }

    #[tokio::test]
    async fn test_audio_processor_lifecycle() {
        let mut processor = RealTimeAudioProcessor::new().unwrap();

        // Initially not running
        assert!(!processor.is_running());

        // Start processing (this may fail if no audio devices available)
        match processor.start_processing().await {
            Ok(_) => {
                assert!(processor.is_running());

                // Let it run briefly
                tokio::time::sleep(Duration::from_millis(50)).await;

                // Stop processing
                processor.stop_processing().await.unwrap();
                assert!(!processor.is_running());
            }
            Err(e) => {
                // Audio device may not be available in test environment
                println!("Audio processing start failed (expected in test environment): {}", e);
            }
        }
    }

    #[test]
    fn test_audio_frame_channel_processing() {
        // Test stereo frame processing
        let stereo_samples = vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6]; // 3 frames, 2 channels
        let mut frame = AudioFrame::new(stereo_samples);

        // Apply gain to left channel only (even indices)
        for i in (0..frame.samples.len()).step_by(2) {
            frame.samples[i] *= 2.0;
        }

        assert_eq!(frame.samples, vec![0.2, 0.2, 0.6, 0.4, 1.0, 0.6]);
    }

    #[test]
    fn test_audio_frame_zero_crossing_rate() {
        // Frame with alternating positive/negative samples
        let frame = AudioFrame::new(vec![1.0, -1.0, 1.0, -1.0, 1.0, -1.0]);
        let zcr = frame.zero_crossing_rate();

        // Should have 5 zero crossings out of 5 possible transitions
        assert_eq!(zcr, 5.0 / 5.0);

        // Frame with no zero crossings
        let frame = AudioFrame::new(vec![1.0, 1.0, 1.0, 1.0]);
        let zcr = frame.zero_crossing_rate();
        assert_eq!(zcr, 0.0);
    }

    #[test]
    fn test_audio_frame_dc_offset() {
        let frame = AudioFrame::new(vec![1.0, 2.0, 3.0, 4.0]);
        let dc_offset = frame.dc_offset();

        // DC offset should be the mean: (1+2+3+4)/4 = 2.5
        assert!((dc_offset - 2.5).abs() < 1e-6);

        // Remove DC offset
        let mut frame_corrected = frame.clone();
        frame_corrected.remove_dc_offset();
        let new_dc = frame_corrected.dc_offset();
        assert!(new_dc.abs() < 1e-6);
    }

    #[test]
    fn test_audio_frame_energy() {
        let frame = AudioFrame::new(vec![1.0, 0.0, -1.0, 0.0]);
        let energy = frame.energy();

        // Energy = sum of squares = 1^2 + 0^2 + (-1)^2 + 0^2 = 2
        assert!((energy - 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_concurrent_frame_processing() {
        use std::sync::Arc;
        use std::thread;

        let frame = Arc::new(AudioFrame::new(vec![0.5; 1000]));
        let frame_clone = Arc::clone(&frame);

        let handle = thread::spawn(move || {
            // Simulate concurrent read-only access
            let _rms = frame_clone.rms();
            let _peak = frame_clone.peak();
            let _energy = frame_clone.energy();
        });

        // Main thread also accesses frame
        let _dc = frame.dc_offset();
        let _zcr = frame.zero_crossing_rate();

        handle.join().unwrap();
    }
}

// Additional implementations for AudioFrame to support comprehensive testing
impl crate::realtime_audio::AudioFrame {
    pub fn normalize(&mut self) {
        let peak = self.peak();
        if peak > 0.0 {
            let scale = 1.0 / peak;
            for sample in &mut self.samples {
                *sample *= scale;
            }
        }
    }

    pub fn apply_gain(&mut self, gain: f32) {
        for sample in &mut self.samples {
            *sample *= gain;
        }
    }

    pub fn mix(&mut self, other: &crate::realtime_audio::AudioFrame, mix_level: f32) {
        let self_level = 1.0 - mix_level;
        for (i, sample) in self.samples.iter_mut().enumerate() {
            if i < other.samples.len() {
                *sample = self_level * *sample + mix_level * other.samples[i];
            }
        }
    }

    pub fn is_silent(&self, threshold: f32) -> bool {
        self.samples.iter().all(|&s| s.abs() < threshold)
    }

    pub fn rms(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f32 = self.samples.iter().map(|&s| s * s).sum();
        (sum_squares / self.samples.len() as f32).sqrt()
    }

    pub fn peak(&self) -> f32 {
        self.samples.iter().map(|&s| s.abs()).fold(0.0, f32::max)
    }

    pub fn zero_crossing_rate(&self) -> f32 {
        if self.samples.len() < 2 {
            return 0.0;
        }

        let mut crossings = 0;
        for i in 1..self.samples.len() {
            if (self.samples[i] >= 0.0) != (self.samples[i-1] >= 0.0) {
                crossings += 1;
            }
        }

        crossings as f32 / (self.samples.len() - 1) as f32
    }

    pub fn dc_offset(&self) -> f32 {
        if self.samples.is_empty() {
            return 0.0;
        }

        self.samples.iter().sum::<f32>() / self.samples.len() as f32
    }

    pub fn remove_dc_offset(&mut self) {
        let dc = self.dc_offset();
        for sample in &mut self.samples {
            *sample -= dc;
        }
    }

    pub fn energy(&self) -> f32 {
        self.samples.iter().map(|&s| s * s).sum()
    }
}