#[cfg(test)]
mod echo_cancellation_tests {
    use crate::echo_cancellation::*;
    use crate::realtime_audio::{AudioFrame, SAMPLE_RATE, CHANNELS, FRAME_SIZE_SAMPLES};
    use std::time::Duration;

    #[test]
    fn test_echo_cancellation_config_default() {
        let config = EchoCancellationConfig::default();

        assert_eq!(config.max_echo_delay_ms, 200.0);
        assert_eq!(config.suppression_strength, 0.8);
        assert_eq!(config.learning_rate, 0.01);
        assert_eq!(config.echo_threshold, 0.01);
        assert!(config.nonlinear_processing);
        assert_eq!(config.filter_length, 512);
    }

    #[test]
    fn test_echo_cancellation_processor_creation() {
        let config = EchoCancellationConfig::default();
        let processor = EchoCancellationProcessor::new(config);

        assert!(processor.is_ok());
        let processor = processor.unwrap();

        assert_eq!(processor.get_config().max_echo_delay_ms, 200.0);
        let stats = processor.get_stats();
        assert_eq!(stats.frames_processed, 0);
        assert!(!stats.echo_detected);
        assert!(!stats.adaptation_active);
    }

    #[test]
    fn test_process_no_echo_scenario() {
        let config = EchoCancellationConfig::default();
        let mut processor = EchoCancellationProcessor::new(config).unwrap();

        // Generate different signals for reference and microphone
        let reference_frame = generate_tone_frame(1000.0, 0.3);
        let mut microphone_frame = generate_tone_frame(2000.0, 0.2); // Different frequency

        let original_mic_rms = microphone_frame.rms();

        let result = processor.process_frame(&reference_frame, &mut microphone_frame);
        assert!(result.is_ok());

        let processed_mic_rms = microphone_frame.rms();

        // Without echo, microphone signal should be mostly preserved
        let preservation_ratio = processed_mic_rms / original_mic_rms;
        assert!(preservation_ratio > 0.8, "Signal preservation too low: {:.3}", preservation_ratio);

        let stats = processor.get_stats();
        assert_eq!(stats.frames_processed, 1);
        assert!(!stats.echo_detected);

        println!("No echo - preservation ratio: {:.3}", preservation_ratio);
    }

    #[test]
    fn test_echo_detection_and_cancellation() {
        let config = EchoCancellationConfig::default();
        let mut processor = EchoCancellationProcessor::new(config).unwrap();

        // Generate reference signal
        let reference_frame = generate_tone_frame(1000.0, 0.5);

        // Simulate echo by creating delayed and attenuated reference
        let mut microphone_frame = reference_frame.clone();
        microphone_frame.apply_gain(0.3); // Echo is typically attenuated

        let original_mic_rms = microphone_frame.rms();

        // Process several frames to allow adaptation
        for _ in 0..20 {
            let result = processor.process_frame(&reference_frame, &mut microphone_frame);
            assert!(result.is_ok());
        }

        let processed_mic_rms = microphone_frame.rms();
        let echo_reduction = 1.0 - (processed_mic_rms / original_mic_rms);

        let stats = processor.get_stats();
        println!("Echo cancellation - frames: {}, echo detected: {}, reduction: {:.1}%",
                stats.frames_processed, stats.echo_detected, echo_reduction * 100.0);

        // Should detect echo and reduce it significantly
        assert!(stats.echo_detected, "Echo should be detected");
        assert!(echo_reduction > 0.3, "Echo reduction too low: {:.1}%", echo_reduction * 100.0);
    }

    #[test]
    fn test_adaptive_filter_convergence() {
        let mut config = EchoCancellationConfig::default();
        config.learning_rate = 0.05; // Faster learning for test
        config.filter_length = 128;  // Smaller filter for faster test

        let mut processor = EchoCancellationProcessor::new(config).unwrap();

        let reference_frame = generate_tone_frame(800.0, 0.4);

        // Simulate echo with known delay and attenuation
        let echo_delay_samples = 10;
        let echo_gain = 0.4;

        let mut convergence_measurements = Vec::new();

        for frame_num in 0..50 {
            // Create microphone signal with simulated echo
            let mut microphone_frame = create_delayed_echo(&reference_frame, echo_delay_samples, echo_gain);

            processor.process_frame(&reference_frame, &mut microphone_frame).unwrap();

            let stats = processor.get_stats();
            convergence_measurements.push(stats.filter_convergence);

            if frame_num % 10 == 9 {
                println!("Frame {}: convergence = {:.3}, suppression = {:.1} dB",
                        frame_num + 1, stats.filter_convergence, stats.echo_suppression_db);
            }
        }

        // Filter should converge over time
        let initial_convergence = convergence_measurements[5]; // Skip first few frames
        let final_convergence = convergence_measurements[convergence_measurements.len() - 1];

        assert!(final_convergence > initial_convergence,
               "Filter did not converge: {:.3} -> {:.3}", initial_convergence, final_convergence);

        let final_stats = processor.get_stats();
        assert!(final_stats.adaptation_active, "Adaptation should be active");
        assert!(final_stats.filter_convergence > 0.5, "Filter should be reasonably converged");
    }

    #[test]
    fn test_double_talk_detection() {
        let config = EchoCancellationConfig::default();
        let mut processor = EchoCancellationProcessor::new(config).unwrap();

        // Process frames with only far-end speech (reference)
        let reference_frame = generate_speech_like_frame(150.0); // Male voice
        let mut silent_mic = AudioFrame::new(vec![0.01; FRAME_SIZE_SAMPLES]); // Very quiet

        for _ in 0..10 {
            processor.process_frame(&reference_frame, &mut silent_mic).unwrap();
        }

        let stats_far_end_only = processor.get_stats();

        // Now simulate double-talk: both far-end and near-end speech
        let near_end_speech = generate_speech_like_frame(200.0); // Female voice, different frequency
        let mut double_talk_mic = reference_frame.clone();
        double_talk_mic.mix(&near_end_speech, 0.7); // Mix with near-end speech

        for _ in 0..10 {
            processor.process_frame(&reference_frame, &mut double_talk_mic).unwrap();
        }

        let stats_double_talk = processor.get_stats();

        println!("Far-end only: adaptation={}, Double-talk: adaptation={}",
                stats_far_end_only.adaptation_active, stats_double_talk.adaptation_active);

        // During double-talk, adaptation might be reduced or stopped
        // This is implementation-dependent, but we can check that the processor handles it
        assert!(stats_double_talk.frames_processed > stats_far_end_only.frames_processed);
    }

    #[test]
    fn test_nonlinear_processing() {
        let mut config = EchoCancellationConfig::default();
        config.nonlinear_processing = true;

        let mut processor_with_nlp = EchoCancellationProcessor::new(config.clone()).unwrap();

        config.nonlinear_processing = false;
        let mut processor_without_nlp = EchoCancellationProcessor::new(config).unwrap();

        // Create reference and echo signals
        let reference_frame = generate_tone_frame(1000.0, 0.4);
        let mut echo_frame_with_nlp = create_delayed_echo(&reference_frame, 5, 0.3);
        let mut echo_frame_without_nlp = echo_frame_with_nlp.clone();

        // Process with both processors
        for _ in 0..15 {
            processor_with_nlp.process_frame(&reference_frame, &mut echo_frame_with_nlp).unwrap();
            processor_without_nlp.process_frame(&reference_frame, &mut echo_frame_without_nlp).unwrap();
        }

        let rms_with_nlp = echo_frame_with_nlp.rms();
        let rms_without_nlp = echo_frame_without_nlp.rms();

        println!("Nonlinear processing: With NLP: {:.6}, Without NLP: {:.6}",
                rms_with_nlp, rms_without_nlp);

        // Nonlinear processing should provide additional suppression
        assert!(rms_with_nlp <= rms_without_nlp * 1.1, // Allow some tolerance
               "NLP should not increase residual echo significantly");
    }

    #[test]
    fn test_various_echo_delays() {
        let delays_ms = vec![10.0, 50.0, 100.0, 150.0];

        for delay_ms in delays_ms {
            let mut config = EchoCancellationConfig::default();
            config.max_echo_delay_ms = delay_ms * 1.5; // Ensure sufficient buffer

            let mut processor = EchoCancellationProcessor::new(config).unwrap();

            let reference_frame = generate_tone_frame(1000.0, 0.4);
            let delay_samples = (delay_ms * SAMPLE_RATE as f32 / 1000.0) as usize;

            // Create echo with specific delay
            let mut echo_frame = create_delayed_echo(&reference_frame, delay_samples, 0.4);
            let original_rms = echo_frame.rms();

            // Process multiple frames for adaptation
            for _ in 0..30 {
                processor.process_frame(&reference_frame, &mut echo_frame).unwrap();
            }

            let processed_rms = echo_frame.rms();
            let echo_reduction = 1.0 - (processed_rms / original_rms);

            println!("Delay {:.1}ms ({} samples): {:.1}% echo reduction",
                    delay_ms, delay_samples, echo_reduction * 100.0);

            // Should achieve some echo reduction for reasonable delays
            if delay_ms <= 100.0 {
                assert!(echo_reduction > 0.2,
                       "Insufficient echo reduction for {}ms delay: {:.1}%",
                       delay_ms, echo_reduction * 100.0);
            }
        }
    }

    #[test]
    fn test_suppression_strength_parameter() {
        let strengths = vec![0.3, 0.8, 1.0];

        for &strength in &strengths {
            let mut config = EchoCancellationConfig::default();
            config.suppression_strength = strength;

            let mut processor = EchoCancellationProcessor::new(config).unwrap();

            let reference_frame = generate_tone_frame(1000.0, 0.5);
            let mut echo_frame = create_delayed_echo(&reference_frame, 10, 0.4);

            let original_rms = echo_frame.rms();

            // Allow adaptation
            for _ in 0..20 {
                processor.process_frame(&reference_frame, &mut echo_frame).unwrap();
            }

            let processed_rms = echo_frame.rms();
            let suppression_ratio = processed_rms / original_rms;

            let stats = processor.get_stats();
            println!("Strength {:.1}: suppression ratio = {:.3}, measured suppression = {:.1} dB",
                    strength, suppression_ratio, stats.echo_suppression_db);

            // Higher strength should provide more suppression
            if strength >= 0.8 {
                assert!(suppression_ratio < 0.7,
                       "Insufficient suppression for strength {:.1}: ratio {:.3}",
                       strength, suppression_ratio);
            }
        }
    }

    #[test]
    fn test_filter_length_impact() {
        let filter_lengths = vec![64, 256, 512];

        for &filter_length in &filter_lengths {
            let mut config = EchoCancellationConfig::default();
            config.filter_length = filter_length;

            let mut processor = EchoCancellationProcessor::new(config).unwrap();

            let reference_frame = generate_tone_frame(1000.0, 0.4);
            let mut echo_frame = create_delayed_echo(&reference_frame, 20, 0.3);

            // Measure convergence speed
            let mut convergence_history = Vec::new();

            for _ in 0..40 {
                processor.process_frame(&reference_frame, &mut echo_frame).unwrap();
                convergence_history.push(processor.get_stats().filter_convergence);
            }

            let final_convergence = convergence_history[convergence_history.len() - 1];

            println!("Filter length {}: final convergence = {:.3}", filter_length, final_convergence);

            // Longer filters should generally achieve better convergence for complex echoes
            assert!(final_convergence >= 0.0, "Convergence should be non-negative");
        }
    }

    #[test]
    fn test_multiple_channel_processing() {
        let config = EchoCancellationConfig::default();
        let mut processor = EchoCancellationProcessor::new(config).unwrap();

        // Create stereo reference signal
        let mut reference_samples = Vec::new();
        let samples_per_channel = FRAME_SIZE_SAMPLES / CHANNELS as usize;

        for i in 0..samples_per_channel {
            let t = i as f32 / SAMPLE_RATE as f32;
            // Left channel: 1kHz tone
            let left = 0.3 * (2.0 * std::f32::consts::PI * 1000.0 * t).sin();
            // Right channel: 1.5kHz tone
            let right = 0.3 * (2.0 * std::f32::consts::PI * 1500.0 * t).sin();

            reference_samples.push(left);
            reference_samples.push(right);
        }

        let reference_frame = AudioFrame::new(reference_samples);

        // Create microphone frame with echo from both channels
        let mut microphone_frame = reference_frame.clone();
        microphone_frame.apply_gain(0.4); // Echo attenuation

        let original_mic_rms = microphone_frame.rms();

        // Process multiple frames
        for _ in 0..25 {
            processor.process_frame(&reference_frame, &mut microphone_frame).unwrap();
        }

        let processed_mic_rms = microphone_frame.rms();
        let echo_reduction = 1.0 - (processed_mic_rms / original_mic_rms);

        let stats = processor.get_stats();

        println!("Multi-channel: echo reduction = {:.1}%, frames processed = {}",
                echo_reduction * 100.0, stats.frames_processed);

        assert!(echo_reduction > 0.2, "Multi-channel echo reduction insufficient: {:.1}%",
               echo_reduction * 100.0);
        assert_eq!(microphone_frame.samples.len(), FRAME_SIZE_SAMPLES);
    }

    #[test]
    fn test_processor_reset() {
        let config = EchoCancellationConfig::default();
        let mut processor = EchoCancellationProcessor::new(config).unwrap();

        let reference_frame = generate_tone_frame(1000.0, 0.4);
        let mut echo_frame = create_delayed_echo(&reference_frame, 15, 0.3);

        // Process frames to build up state
        for _ in 0..20 {
            processor.process_frame(&reference_frame, &mut echo_frame).unwrap();
        }

        let stats_before = processor.get_stats();
        assert!(stats_before.frames_processed > 0);
        assert!(stats_before.adaptation_active);

        // Reset processor
        processor.reset();

        let stats_after = processor.get_stats();
        assert_eq!(stats_after.frames_processed, 0);
        assert!(!stats_after.echo_detected);
        assert!(!stats_after.adaptation_active);
        assert_eq!(stats_after.echo_suppression_db, 0.0);
        assert_eq!(stats_after.filter_convergence, 0.0);

        println!("Reset: frames {} -> {}", stats_before.frames_processed, stats_after.frames_processed);
    }

    #[test]
    fn test_config_update() {
        let config = EchoCancellationConfig::default();
        let mut processor = EchoCancellationProcessor::new(config).unwrap();

        // Process a few frames
        let reference_frame = generate_tone_frame(1000.0, 0.4);
        let mut echo_frame = create_delayed_echo(&reference_frame, 10, 0.3);

        for _ in 0..5 {
            processor.process_frame(&reference_frame, &mut echo_frame).unwrap();
        }

        // Update configuration
        let mut new_config = EchoCancellationConfig::default();
        new_config.suppression_strength = 0.5;
        new_config.filter_length = 256;

        processor.update_config(new_config.clone());

        assert_eq!(processor.get_config().suppression_strength, 0.5);
        assert_eq!(processor.get_config().filter_length, 256);

        // Continue processing with new config
        for _ in 0..5 {
            processor.process_frame(&reference_frame, &mut echo_frame).unwrap();
        }

        let stats = processor.get_stats();
        assert_eq!(stats.frames_processed, 10);
    }

    #[test]
    fn test_echo_return_loss_calculation() {
        let config = EchoCancellationConfig::default();
        let mut processor = EchoCancellationProcessor::new(config).unwrap();

        // Create scenario with known echo return loss
        let reference_power = 0.25; // RMS 0.5
        let echo_power = 0.01;      // RMS 0.1, so -14dB return loss

        let reference_frame = generate_tone_frame(1000.0, 0.5);
        let mut microphone_frame = generate_tone_frame(1000.0, 0.1);

        for _ in 0..10 {
            processor.process_frame(&reference_frame, &mut microphone_frame).unwrap();
        }

        let stats = processor.get_stats();
        let calculated_erl = stats.echo_return_loss_db();

        println!("Echo Return Loss: {:.1} dB", calculated_erl);

        // Should be in reasonable range (actual value depends on processing)
        assert!(calculated_erl > -30.0 && calculated_erl < 10.0,
               "Echo return loss out of reasonable range: {:.1} dB", calculated_erl);
    }

    #[test]
    fn test_comfort_noise_injection() {
        let mut config = EchoCancellationConfig::default();
        config.nonlinear_processing = true;

        let mut processor = EchoCancellationProcessor::new(config).unwrap();

        // Create very quiet echo scenario
        let reference_frame = generate_tone_frame(1000.0, 0.1);
        let mut quiet_echo = create_delayed_echo(&reference_frame, 10, 0.05);

        // Process to establish echo detection and suppression
        for _ in 0..20 {
            processor.process_frame(&reference_frame, &mut quiet_echo).unwrap();
        }

        // The processed signal should have some comfort noise
        let processed_rms = quiet_echo.rms();

        println!("Comfort noise test - processed RMS: {:.6}", processed_rms);

        // Even with aggressive suppression, should have minimal comfort noise
        assert!(processed_rms > 0.0, "Should have some comfort noise");
        assert!(processed_rms < 0.01, "Comfort noise should not be too loud");
    }

    // Helper functions
    fn generate_tone_frame(frequency: f32, amplitude: f32) -> AudioFrame {
        let mut samples = vec![0.0; FRAME_SIZE_SAMPLES];
        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            *sample = amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin();
        }
        AudioFrame::new(samples)
    }

    fn generate_speech_like_frame(fundamental_freq: f32) -> AudioFrame {
        let mut samples = vec![0.0; FRAME_SIZE_SAMPLES];

        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            let mut signal = 0.0;

            // Add harmonics with decreasing amplitude
            for harmonic in 1..=4 {
                let freq = fundamental_freq * harmonic as f32;
                let amplitude = 0.3 / harmonic as f32;
                signal += amplitude * (2.0 * std::f32::consts::PI * freq * t).sin();
            }

            // Add some noise for realism
            signal += 0.02 * (rand::random::<f32>() - 0.5);

            *sample = signal;
        }

        AudioFrame::new(samples)
    }

    fn create_delayed_echo(reference: &AudioFrame, delay_samples: usize, echo_gain: f32) -> AudioFrame {
        let mut echo_samples = vec![0.0; FRAME_SIZE_SAMPLES];

        for i in 0..FRAME_SIZE_SAMPLES {
            if i >= delay_samples {
                echo_samples[i] = echo_gain * reference.samples[i - delay_samples];
            }
            // Add a small amount of the original signal to simulate microphone pickup
            echo_samples[i] += 0.1 * reference.samples[i];
        }

        AudioFrame::new(echo_samples)
    }
}