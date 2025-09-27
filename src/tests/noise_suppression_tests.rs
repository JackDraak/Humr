#[cfg(test)]
mod noise_suppression_tests {
    use super::super::noise_suppression::*;
    use super::super::realtime_audio::{AudioFrame, SAMPLE_RATE, CHANNELS, FRAME_SIZE_SAMPLES};
    use std::collections::VecDeque;

    #[test]
    fn test_noise_suppression_config_default() {
        let config = NoiseSuppressionConfig::default();

        assert_eq!(config.strength, 0.7);
        assert_eq!(config.noise_floor_db, -50.0);
        assert_eq!(config.attack_time_ms, 5.0);
        assert_eq!(config.release_time_ms, 50.0);
        assert_eq!(config.spectral_subtraction_factor, 2.0);
        assert!(config.adaptive);
    }

    #[test]
    fn test_noise_suppression_processor_creation() {
        let config = NoiseSuppressionConfig::default();
        let processor = NoiseSuppressionProcessor::new(config);

        assert!(processor.is_ok());
        let processor = processor.unwrap();

        assert_eq!(processor.get_config().strength, 0.7);
        assert_eq!(processor.get_stats().frames_processed, 0);
        assert_eq!(processor.get_stats().gate_state, GateState::Closed);
    }

    #[test]
    fn test_process_silent_frame() {
        let config = NoiseSuppressionConfig::default();
        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        let mut silent_frame = AudioFrame::new(vec![0.0; FRAME_SIZE_SAMPLES]);
        let result = processor.process_frame(&mut silent_frame);

        assert!(result.is_ok());
        assert_eq!(processor.get_stats().frames_processed, 1);

        // Silent frame should remain silent
        for &sample in &silent_frame.samples {
            assert!(sample.abs() < 1e-6);
        }
    }

    #[test]
    fn test_process_noise_frame() {
        let config = NoiseSuppressionConfig::default();
        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        // Generate white noise
        let mut noise_frame = generate_white_noise_frame(0.1); // Low amplitude noise

        let original_rms = noise_frame.rms();
        let result = processor.process_frame(&mut noise_frame);

        assert!(result.is_ok());

        let processed_rms = noise_frame.rms();

        // Noise should be reduced
        assert!(processed_rms <= original_rms);
        println!("Noise RMS: {:.6} -> {:.6} (reduction: {:.1}%)",
                original_rms, processed_rms,
                (1.0 - processed_rms / original_rms) * 100.0);
    }

    #[test]
    fn test_process_speech_signal() {
        let config = NoiseSuppressionConfig::default();
        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        // Generate speech-like signal (should be preserved)
        let mut speech_frame = generate_speech_like_frame();

        let original_rms = speech_frame.rms();
        let result = processor.process_frame(&mut speech_frame);

        assert!(result.is_ok());

        let processed_rms = speech_frame.rms();

        // Speech should be mostly preserved
        let preservation_ratio = processed_rms / original_rms;
        assert!(preservation_ratio > 0.7, "Speech preservation ratio too low: {:.3}", preservation_ratio);

        println!("Speech RMS: {:.6} -> {:.6} (preservation: {:.1}%)",
                original_rms, processed_rms, preservation_ratio * 100.0);
    }

    #[test]
    fn test_adaptive_noise_estimation() {
        let mut config = NoiseSuppressionConfig::default();
        config.adaptive = true;

        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        // Process several frames of noise to build noise estimate
        for _ in 0..20 {
            let mut noise_frame = generate_white_noise_frame(0.05);
            processor.process_frame(&mut noise_frame).unwrap();
        }

        let stats_after_noise = processor.get_stats();
        let noise_floor_after_noise = stats_after_noise.noise_floor_estimate;

        // Now process a loud signal
        let mut signal_frame = generate_tone_frame(1000.0, 0.5);
        processor.process_frame(&mut signal_frame).unwrap();

        let stats_after_signal = processor.get_stats();

        // Noise floor should not increase significantly due to signal
        assert!(stats_after_signal.noise_floor_estimate <= noise_floor_after_noise * 2.0,
               "Noise floor increased too much: {:.6} -> {:.6}",
               noise_floor_after_noise, stats_after_signal.noise_floor_estimate);

        println!("Adaptive noise floor: {:.6} -> {:.6}",
                noise_floor_after_noise, stats_after_signal.noise_floor_estimate);
    }

    #[test]
    fn test_spectral_subtraction_effectiveness() {
        let mut config = NoiseSuppressionConfig::default();
        config.spectral_subtraction_factor = 3.0; // Aggressive subtraction

        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        // Train on noise
        for _ in 0..10 {
            let mut noise_frame = generate_white_noise_frame(0.1);
            processor.process_frame(&mut noise_frame).unwrap();
        }

        // Test on mixed signal (signal + noise)
        let signal = generate_tone_frame(800.0, 0.3);
        let noise = generate_white_noise_frame(0.1);
        let mut mixed_frame = mix_frames(&signal, &noise, 1.0, 1.0);

        let original_snr = calculate_snr(&signal, &mixed_frame);

        processor.process_frame(&mut mixed_frame).unwrap();

        let processed_snr = calculate_snr(&signal, &mixed_frame);

        println!("SNR improvement: {:.2} dB -> {:.2} dB (gain: {:.2} dB)",
                original_snr, processed_snr, processed_snr - original_snr);

        // SNR should improve
        assert!(processed_snr > original_snr,
               "SNR did not improve: {:.2} -> {:.2}", original_snr, processed_snr);
    }

    #[test]
    fn test_noise_gate_functionality() {
        let mut config = NoiseSuppressionConfig::default();
        config.noise_floor_db = -30.0; // Higher threshold for testing

        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        // Test with signal below gate threshold
        let mut quiet_frame = generate_tone_frame(1000.0, 0.01); // Very quiet
        processor.process_frame(&mut quiet_frame).unwrap();

        let stats_quiet = processor.get_stats();
        assert_eq!(stats_quiet.gate_state, GateState::Closed);

        // Test with signal above gate threshold
        let mut loud_frame = generate_tone_frame(1000.0, 0.3); // Loud
        processor.process_frame(&mut loud_frame).unwrap();

        let stats_loud = processor.get_stats();
        assert!(matches!(stats_loud.gate_state, GateState::Open | GateState::Attack));

        println!("Gate states: quiet={:?}, loud={:?}",
                stats_quiet.gate_state, stats_loud.gate_state);
    }

    #[test]
    fn test_strength_parameter_effect() {
        let strengths = vec![0.0, 0.5, 1.0];

        for &strength in &strengths {
            let mut config = NoiseSuppressionConfig::default();
            config.strength = strength;

            let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

            // Train on noise
            for _ in 0..5 {
                let mut noise_frame = generate_white_noise_frame(0.1);
                processor.process_frame(&mut noise_frame).unwrap();
            }

            // Test noise suppression
            let mut noise_frame = generate_white_noise_frame(0.1);
            let original_rms = noise_frame.rms();

            processor.process_frame(&mut noise_frame).unwrap();

            let processed_rms = noise_frame.rms();
            let suppression_ratio = processed_rms / original_rms;

            println!("Strength {:.1}: suppression ratio = {:.3}", strength, suppression_ratio);

            // Higher strength should mean more suppression (lower ratio)
            if strength == 0.0 {
                assert!(suppression_ratio > 0.9, "No suppression expected at strength 0");
            } else if strength == 1.0 {
                assert!(suppression_ratio < 0.5, "Strong suppression expected at strength 1");
            }
        }
    }

    #[test]
    fn test_attack_release_times() {
        let mut config = NoiseSuppressionConfig::default();
        config.attack_time_ms = 1.0;   // Very fast attack
        config.release_time_ms = 100.0; // Slow release

        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        // Start with quiet signal
        let mut quiet_frame = generate_tone_frame(1000.0, 0.01);
        processor.process_frame(&mut quiet_frame).unwrap();
        assert_eq!(processor.get_stats().gate_state, GateState::Closed);

        // Sudden loud signal should trigger fast attack
        let mut loud_frame = generate_tone_frame(1000.0, 0.5);
        processor.process_frame(&mut loud_frame).unwrap();

        let state_after_loud = processor.get_stats().gate_state;
        assert!(matches!(state_after_loud, GateState::Open | GateState::Attack));

        // Return to quiet - should take longer to close due to slow release
        let mut quiet_frame2 = generate_tone_frame(1000.0, 0.01);
        processor.process_frame(&mut quiet_frame2).unwrap();

        let state_after_quiet = processor.get_stats().gate_state;
        // Might still be open or in release due to slow release time
        println!("Gate state after returning to quiet: {:?}", state_after_quiet);
    }

    #[test]
    fn test_frequency_domain_processing() {
        let config = NoiseSuppressionConfig::default();
        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        // Generate signal with specific frequency content
        let signal1 = generate_tone_frame(1000.0, 0.3); // 1kHz tone
        let signal2 = generate_tone_frame(3000.0, 0.3); // 3kHz tone

        let mut mixed_frame = mix_frames(&signal1, &signal2, 1.0, 1.0);

        // Train on noise that doesn't contain these frequencies
        for _ in 0..10 {
            let mut noise_frame = generate_white_noise_frame(0.05);
            processor.process_frame(&mut noise_frame).unwrap();
        }

        let original_energy = mixed_frame.energy();
        processor.process_frame(&mut mixed_frame).unwrap();
        let processed_energy = mixed_frame.energy();

        // Signal should be mostly preserved
        let energy_ratio = processed_energy / original_energy;
        assert!(energy_ratio > 0.7, "Too much signal energy lost: {:.3}", energy_ratio);

        println!("Energy preservation: {:.1}%", energy_ratio * 100.0);
    }

    #[test]
    fn test_multi_channel_processing() {
        let config = NoiseSuppressionConfig::default();
        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        // Generate stereo frame with different content in each channel
        let mut stereo_samples = Vec::new();
        let samples_per_channel = FRAME_SIZE_SAMPLES / CHANNELS as usize;

        for i in 0..samples_per_channel {
            // Left channel: 1kHz tone
            let t = i as f32 / SAMPLE_RATE as f32;
            let left = 0.3 * (2.0 * std::f32::consts::PI * 1000.0 * t).sin();

            // Right channel: white noise
            let right = 0.1 * (rand::random::<f32>() - 0.5);

            stereo_samples.push(left);
            stereo_samples.push(right);
        }

        let mut stereo_frame = AudioFrame::new(stereo_samples);

        // Process several frames to establish noise estimates
        for _ in 0..10 {
            let mut frame_copy = stereo_frame.clone();
            processor.process_frame(&mut frame_copy).unwrap();
        }

        let result = processor.process_frame(&mut stereo_frame);
        assert!(result.is_ok());

        // Verify frame length is preserved
        assert_eq!(stereo_frame.samples.len(), FRAME_SIZE_SAMPLES);

        // Extract channels for analysis
        let mut left_samples = Vec::new();
        let mut right_samples = Vec::new();

        for i in 0..samples_per_channel {
            left_samples.push(stereo_frame.samples[i * 2]);
            right_samples.push(stereo_frame.samples[i * 2 + 1]);
        }

        let left_rms = calculate_rms(&left_samples);
        let right_rms = calculate_rms(&right_samples);

        println!("Channel RMS - Left: {:.6}, Right: {:.6}", left_rms, right_rms);

        // Both channels should have some content, but right (noise) should be more suppressed
        assert!(left_rms > 0.0 && right_rms > 0.0);
    }

    #[test]
    fn test_processor_reset() {
        let config = NoiseSuppressionConfig::default();
        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        // Process some frames to build up state
        for i in 0..10 {
            let mut frame = generate_tone_frame(1000.0 + i as f32 * 100.0, 0.2);
            processor.process_frame(&mut frame).unwrap();
        }

        let stats_before = processor.get_stats();
        assert!(stats_before.frames_processed > 0);

        // Reset processor
        processor.reset();

        let stats_after = processor.get_stats();
        assert_eq!(stats_after.frames_processed, 0);
        assert_eq!(stats_after.gate_state, GateState::Closed);
        assert_eq!(stats_after.noise_reduction_applied, 0.0);

        println!("Reset: frames {} -> {}", stats_before.frames_processed, stats_after.frames_processed);
    }

    #[test]
    fn test_real_time_performance() {
        let config = NoiseSuppressionConfig::default();
        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        let num_frames = 100;
        let start_time = std::time::Instant::now();

        for i in 0..num_frames {
            let mut frame = if i % 2 == 0 {
                generate_tone_frame(1000.0, 0.3)
            } else {
                generate_white_noise_frame(0.1)
            };

            processor.process_frame(&mut frame).unwrap();
        }

        let elapsed = start_time.elapsed();
        let frames_per_second = num_frames as f32 / elapsed.as_secs_f32();
        let real_time_ratio = frames_per_second / 50.0; // 50 fps for 20ms frames

        println!("Performance: {:.1} fps ({:.1}x real-time)", frames_per_second, real_time_ratio);

        // Should be significantly faster than real-time
        assert!(real_time_ratio > 10.0, "Processing too slow: {:.1}x real-time", real_time_ratio);
    }

    #[test]
    fn test_edge_cases() {
        let config = NoiseSuppressionConfig::default();
        let mut processor = NoiseSuppressionProcessor::new(config).unwrap();

        // Test with all zeros
        let mut zero_frame = AudioFrame::new(vec![0.0; FRAME_SIZE_SAMPLES]);
        assert!(processor.process_frame(&mut zero_frame).is_ok());

        // Test with clipped signal
        let mut clipped_frame = AudioFrame::new(vec![1.0; FRAME_SIZE_SAMPLES]);
        assert!(processor.process_frame(&mut clipped_frame).is_ok());

        // Test with very small values
        let mut tiny_frame = AudioFrame::new(vec![1e-10; FRAME_SIZE_SAMPLES]);
        assert!(processor.process_frame(&mut tiny_frame).is_ok());

        // Test with alternating max values
        let mut alternating = Vec::new();
        for i in 0..FRAME_SIZE_SAMPLES {
            alternating.push(if i % 2 == 0 { 1.0 } else { -1.0 });
        }
        let mut alternating_frame = AudioFrame::new(alternating);
        assert!(processor.process_frame(&mut alternating_frame).is_ok());
    }

    // Helper functions
    fn generate_white_noise_frame(amplitude: f32) -> AudioFrame {
        let mut samples = vec![0.0; FRAME_SIZE_SAMPLES];
        for sample in &mut samples {
            *sample = amplitude * (rand::random::<f32>() - 0.5) * 2.0;
        }
        AudioFrame::new(samples)
    }

    fn generate_tone_frame(frequency: f32, amplitude: f32) -> AudioFrame {
        let mut samples = vec![0.0; FRAME_SIZE_SAMPLES];
        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            *sample = amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin();
        }
        AudioFrame::new(samples)
    }

    fn generate_speech_like_frame() -> AudioFrame {
        let mut samples = vec![0.0; FRAME_SIZE_SAMPLES];
        let f0 = 150.0; // Fundamental frequency

        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            let mut signal = 0.0;

            // Add harmonics
            for harmonic in 1..=5 {
                let freq = f0 * harmonic as f32;
                let amplitude = 0.2 / harmonic as f32;
                signal += amplitude * (2.0 * std::f32::consts::PI * freq * t).sin();
            }

            // Add formant-like resonances
            signal += 0.1 * (2.0 * std::f32::consts::PI * 800.0 * t).sin(); // First formant
            signal += 0.05 * (2.0 * std::f32::consts::PI * 1200.0 * t).sin(); // Second formant

            *sample = signal;
        }

        AudioFrame::new(samples)
    }

    fn mix_frames(frame1: &AudioFrame, frame2: &AudioFrame, gain1: f32, gain2: f32) -> AudioFrame {
        let mut mixed = frame1.clone();
        for (i, sample) in mixed.samples.iter_mut().enumerate() {
            *sample = gain1 * *sample + gain2 * frame2.samples.get(i).unwrap_or(&0.0);
        }
        mixed
    }

    fn calculate_snr(signal: &AudioFrame, noisy: &AudioFrame) -> f32 {
        if signal.samples.len() != noisy.samples.len() {
            return 0.0;
        }

        let mut signal_power = 0.0;
        let mut noise_power = 0.0;

        for (sig, noisy) in signal.samples.iter().zip(noisy.samples.iter()) {
            signal_power += sig * sig;
            let noise = noisy - sig;
            noise_power += noise * noise;
        }

        if noise_power == 0.0 {
            return 100.0;
        }

        10.0 * (signal_power / noise_power).log10()
    }

    fn calculate_rms(samples: &[f32]) -> f32 {
        if samples.is_empty() {
            return 0.0;
        }

        let sum_squares: f32 = samples.iter().map(|&s| s * s).sum();
        (sum_squares / samples.len() as f32).sqrt()
    }
}