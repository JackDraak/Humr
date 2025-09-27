#[cfg(test)]
mod opus_codec_tests {
    use super::super::opus_codec::*;
    use super::super::realtime_audio::{AudioFrame, SAMPLE_RATE, CHANNELS, FRAME_SIZE_SAMPLES};
    use std::time::Instant;

    #[test]
    fn test_opus_config_default() {
        let config = OpusConfig::default();

        assert_eq!(config.sample_rate, SAMPLE_RATE);
        assert_eq!(config.channels, CHANNELS);
        assert_eq!(config.bitrate, 64000);
        assert_eq!(config.frame_size_ms, 20);
        assert_eq!(config.complexity, 5);
        assert_eq!(config.application, OpusApplication::VoIP);
        assert!(config.fec_enabled);
        assert!(config.dtx_enabled);
    }

    #[test]
    fn test_opus_config_validation() {
        let mut config = OpusConfig::default();

        // Test valid configurations
        config.sample_rate = 48000;
        assert!(config.validate().is_ok());

        config.sample_rate = 24000;
        assert!(config.validate().is_ok());

        // Test invalid sample rate
        config.sample_rate = 22050;
        assert!(config.validate().is_err());

        // Reset to valid and test channels
        config.sample_rate = 48000;
        config.channels = 1;
        assert!(config.validate().is_ok());

        config.channels = 2;
        assert!(config.validate().is_ok());

        config.channels = 3;
        assert!(config.validate().is_err());

        // Test bitrate bounds
        config.channels = 2;
        config.bitrate = 6000;
        assert!(config.validate().is_ok());

        config.bitrate = 5000;
        assert!(config.validate().is_err());

        config.bitrate = 512000;
        assert!(config.validate().is_ok());

        config.bitrate = 600000;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_opus_codec_creation() {
        let config = OpusConfig::default();
        let codec = OpusCodec::new(config);

        assert!(codec.is_ok());
        let codec = codec.unwrap();

        assert_eq!(codec.get_config().sample_rate, SAMPLE_RATE);
        assert_eq!(codec.get_config().channels, CHANNELS);
    }

    #[test]
    fn test_opus_encode_decode_silence() {
        let config = OpusConfig::default();
        let mut codec = OpusCodec::new(config).unwrap();

        // Create silent frame
        let silent_frame = AudioFrame::new(vec![0.0; FRAME_SIZE_SAMPLES]);

        // Encode
        let encoded_result = codec.encode(&silent_frame);
        assert!(encoded_result.is_ok());

        let encoded_data = encoded_result.unwrap();
        assert!(!encoded_data.is_empty());
        println!("Encoded silence: {} bytes", encoded_data.len());

        // Decode
        let decoded_result = codec.decode(&encoded_data);
        assert!(decoded_result.is_ok());

        let decoded_frame = decoded_result.unwrap();
        assert_eq!(decoded_frame.samples.len(), FRAME_SIZE_SAMPLES);

        // Decoded silence should be very quiet
        let peak = decoded_frame.peak();
        assert!(peak < 0.01, "Decoded silence peak too high: {}", peak);
    }

    #[test]
    fn test_opus_encode_decode_tone() {
        let config = OpusConfig::default();
        let mut codec = OpusCodec::new(config).unwrap();

        // Generate 1kHz sine wave
        let mut samples = vec![0.0; FRAME_SIZE_SAMPLES];
        let frequency = 1000.0;
        let amplitude = 0.5;

        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            *sample = amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin();
        }

        let original_frame = AudioFrame::new(samples);
        let original_peak = original_frame.peak();

        // Encode
        let encoded_data = codec.encode(&original_frame).unwrap();
        assert!(!encoded_data.is_empty());
        println!("Encoded 1kHz tone: {} bytes", encoded_data.len());

        // Decode
        let decoded_frame = codec.decode(&encoded_data).unwrap();
        let decoded_peak = decoded_frame.peak();

        // Should preserve most of the signal energy
        let energy_ratio = decoded_peak / original_peak;
        assert!(energy_ratio > 0.8, "Energy ratio too low: {}", energy_ratio);
        assert!(energy_ratio < 1.2, "Energy ratio too high: {}", energy_ratio);

        // Check frame length
        assert_eq!(decoded_frame.samples.len(), FRAME_SIZE_SAMPLES);
    }

    #[test]
    fn test_opus_compression_efficiency() {
        let config = OpusConfig::default();
        let mut codec = OpusCodec::new(config).unwrap();

        // Create frame with repeating pattern (should compress well)
        let pattern = vec![0.1, 0.2, 0.1, 0.2];
        let mut samples = Vec::new();
        for _ in 0..(FRAME_SIZE_SAMPLES / 4) {
            samples.extend_from_slice(&pattern);
        }
        samples.resize(FRAME_SIZE_SAMPLES, 0.0);

        let frame = AudioFrame::new(samples);

        // Encode
        let encoded_data = codec.encode(&frame).unwrap();

        // Check compression ratio
        let uncompressed_size = FRAME_SIZE_SAMPLES * 4; // 4 bytes per f32
        let compression_ratio = uncompressed_size as f32 / encoded_data.len() as f32;

        println!("Compression ratio: {:.2}:1 ({} -> {} bytes)",
                compression_ratio, uncompressed_size, encoded_data.len());

        assert!(compression_ratio > 5.0, "Compression ratio too low: {}", compression_ratio);
    }

    #[test]
    fn test_opus_different_bitrates() {
        let bitrates = vec![16000, 32000, 64000, 128000];

        for bitrate in bitrates {
            let mut config = OpusConfig::default();
            config.bitrate = bitrate;

            let mut codec = OpusCodec::new(config).unwrap();

            // Generate test signal
            let samples = generate_test_signal(FRAME_SIZE_SAMPLES);
            let frame = AudioFrame::new(samples);

            // Encode
            let encoded_data = codec.encode(&frame).unwrap();

            println!("Bitrate {}: {} bytes encoded", bitrate, encoded_data.len());

            // Decode
            let decoded_frame = codec.decode(&encoded_data).unwrap();
            assert_eq!(decoded_frame.samples.len(), FRAME_SIZE_SAMPLES);

            // Higher bitrates should generally produce larger encoded data
            // (though this isn't strictly guaranteed for all signals)
        }
    }

    #[test]
    fn test_opus_voip_vs_audio_application() {
        let signals = vec![
            ("speech_like", generate_speech_like_signal(FRAME_SIZE_SAMPLES)),
            ("music_like", generate_music_like_signal(FRAME_SIZE_SAMPLES)),
        ];

        let applications = vec![
            (OpusApplication::VoIP, "VoIP"),
            (OpusApplication::Audio, "Audio"),
        ];

        for (signal_name, signal) in signals {
            for (app_type, app_name) in &applications {
                let mut config = OpusConfig::default();
                config.application = *app_type;

                let mut codec = OpusCodec::new(config).unwrap();
                let frame = AudioFrame::new(signal.clone());

                let encoded_data = codec.encode(&frame).unwrap();
                let decoded_frame = codec.decode(&encoded_data).unwrap();

                // Calculate quality metric (simplified SNR)
                let snr = calculate_snr(&frame, &decoded_frame);

                println!("{} with {}: {} bytes, SNR: {:.2} dB",
                        signal_name, app_name, encoded_data.len(), snr);

                // Both should produce reasonable quality
                assert!(snr > 20.0, "SNR too low for {} with {}: {}", signal_name, app_name, snr);
            }
        }
    }

    #[test]
    fn test_opus_fec_functionality() {
        let mut config = OpusConfig::default();
        config.fec_enabled = true;

        let mut codec = OpusCodec::new(config).unwrap();

        // Generate test signal
        let samples = generate_test_signal(FRAME_SIZE_SAMPLES);
        let frame = AudioFrame::new(samples);

        // Encode with FEC enabled
        let encoded_data = codec.encode(&frame).unwrap();

        // Decode normally
        let decoded_frame = codec.decode(&encoded_data).unwrap();
        assert_eq!(decoded_frame.samples.len(), FRAME_SIZE_SAMPLES);

        // Test packet loss simulation (this would require more advanced Opus API)
        // For now, just verify FEC doesn't break normal operation
        let quality = calculate_snr(&frame, &decoded_frame);
        assert!(quality > 25.0, "Quality degraded with FEC: {}", quality);
    }

    #[test]
    fn test_opus_dtx_functionality() {
        let mut config = OpusConfig::default();
        config.dtx_enabled = true;

        let mut codec = OpusCodec::new(config).unwrap();

        // Test with silence (should activate DTX)
        let silent_frame = AudioFrame::new(vec![0.0; FRAME_SIZE_SAMPLES]);
        let silent_encoded = codec.encode(&silent_frame).unwrap();

        // Test with signal
        let signal_frame = AudioFrame::new(generate_test_signal(FRAME_SIZE_SAMPLES));
        let signal_encoded = codec.encode(&signal_frame).unwrap();

        println!("DTX - Silent: {} bytes, Signal: {} bytes",
                silent_encoded.len(), signal_encoded.len());

        // DTX should produce smaller packets for silence
        assert!(silent_encoded.len() <= signal_encoded.len(),
               "DTX not working: silence {} bytes >= signal {} bytes",
               silent_encoded.len(), signal_encoded.len());
    }

    #[test]
    fn test_opus_complexity_levels() {
        let complexities = vec![0, 5, 10];

        for complexity in complexities {
            let mut config = OpusConfig::default();
            config.complexity = complexity;

            let mut codec = OpusCodec::new(config).unwrap();

            let samples = generate_test_signal(FRAME_SIZE_SAMPLES);
            let frame = AudioFrame::new(samples);

            let start_time = Instant::now();
            let encoded_data = codec.encode(&frame).unwrap();
            let encode_time = start_time.elapsed();

            let decoded_frame = codec.decode(&encoded_data).unwrap();

            let quality = calculate_snr(&frame, &decoded_frame);

            println!("Complexity {}: {} bytes, {:.2} dB SNR, {:.2}ms encode time",
                    complexity, encoded_data.len(), quality, encode_time.as_millis());

            // Higher complexity should not significantly degrade quality
            assert!(quality > 25.0, "Quality too low for complexity {}: {}", complexity, quality);
        }
    }

    #[test]
    fn test_opus_frame_size_variations() {
        // Test different frame sizes supported by Opus
        let frame_sizes_ms = vec![10, 20, 40];

        for frame_size_ms in frame_sizes_ms {
            let mut config = OpusConfig::default();
            config.frame_size_ms = frame_size_ms;

            let mut codec = OpusCodec::new(config).unwrap();

            let frame_samples = (SAMPLE_RATE as f32 * frame_size_ms as f32 / 1000.0) as usize;
            let samples = generate_test_signal(frame_samples);
            let frame = AudioFrame::new(samples);

            let encoded_data = codec.encode(&frame).unwrap();
            let decoded_frame = codec.decode(&encoded_data).unwrap();

            assert_eq!(decoded_frame.samples.len(), frame_samples);

            println!("Frame size {}ms: {} samples, {} bytes encoded",
                    frame_size_ms, frame_samples, encoded_data.len());
        }
    }

    #[test]
    fn test_opus_error_handling() {
        let config = OpusConfig::default();
        let mut codec = OpusCodec::new(config).unwrap();

        // Test decoding invalid data
        let invalid_data = vec![0xFF, 0xFF, 0xFF, 0xFF];
        let result = codec.decode(&invalid_data);
        assert!(result.is_err());

        // Test encoding invalid frame size
        let wrong_size_frame = AudioFrame::new(vec![0.0; 100]); // Wrong size
        let result = codec.encode(&wrong_size_frame);
        // This might work (Opus can handle different sizes) or fail gracefully
        match result {
            Ok(data) => println!("Opus handled wrong frame size gracefully: {} bytes", data.len()),
            Err(e) => println!("Opus correctly rejected wrong frame size: {}", e),
        }
    }

    #[test]
    fn test_opus_statistics() {
        let config = OpusConfig::default();
        let mut codec = OpusCodec::new(config).unwrap();

        // Encode several frames
        for i in 0..10 {
            let samples = generate_test_signal_with_offset(FRAME_SIZE_SAMPLES, i as f32 * 0.1);
            let frame = AudioFrame::new(samples);

            let encoded_data = codec.encode(&frame).unwrap();
            let _decoded_frame = codec.decode(&encoded_data).unwrap();
        }

        let stats = codec.get_stats();
        assert_eq!(stats.frames_encoded, 10);
        assert_eq!(stats.frames_decoded, 10);
        assert_eq!(stats.encoding_errors, 0);
        assert_eq!(stats.decoding_errors, 0);
        assert!(stats.total_bytes_encoded > 0);
        assert!(stats.average_compression_ratio > 1.0);

        println!("Codec stats: {:#?}", stats);
    }

    #[test]
    fn test_concurrent_codec_usage() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let config = OpusConfig::default();
        let codec = Arc::new(Mutex::new(OpusCodec::new(config).unwrap()));

        let mut handles = vec![];

        // Spawn multiple threads using the codec
        for thread_id in 0..4 {
            let codec_clone = Arc::clone(&codec);

            let handle = thread::spawn(move || {
                for i in 0..5 {
                    let samples = generate_test_signal_with_offset(FRAME_SIZE_SAMPLES,
                                                                 (thread_id * 5 + i) as f32 * 0.1);
                    let frame = AudioFrame::new(samples);

                    let mut codec_guard = codec_clone.lock().unwrap();
                    let encoded_data = codec_guard.encode(&frame).unwrap();
                    let _decoded_frame = codec_guard.decode(&encoded_data).unwrap();
                    drop(codec_guard);

                    // Small delay to allow other threads
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        let codec_guard = codec.lock().unwrap();
        let stats = codec_guard.get_stats();
        assert_eq!(stats.frames_encoded, 20); // 4 threads * 5 frames each
        assert_eq!(stats.frames_decoded, 20);
    }

    // Helper functions for test signal generation
    fn generate_test_signal(length: usize) -> Vec<f32> {
        let mut samples = vec![0.0; length];
        let frequency = 440.0; // A4 note
        let amplitude = 0.3;

        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            *sample = amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin();
        }

        samples
    }

    fn generate_test_signal_with_offset(length: usize, phase_offset: f32) -> Vec<f32> {
        let mut samples = vec![0.0; length];
        let frequency = 440.0;
        let amplitude = 0.3;

        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            *sample = amplitude * (2.0 * std::f32::consts::PI * frequency * t + phase_offset).sin();
        }

        samples
    }

    fn generate_speech_like_signal(length: usize) -> Vec<f32> {
        let mut samples = vec![0.0; length];

        // Simulate speech with multiple harmonics and noise
        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;

            // Fundamental frequency around 150Hz (typical male voice)
            let f0 = 150.0;
            let mut signal = 0.0;

            // Add harmonics with decreasing amplitude
            for harmonic in 1..=5 {
                let freq = f0 * harmonic as f32;
                let amplitude = 0.2 / harmonic as f32;
                signal += amplitude * (2.0 * std::f32::consts::PI * freq * t).sin();
            }

            // Add some noise
            signal += 0.05 * (rand::random::<f32>() - 0.5);

            *sample = signal;
        }

        samples
    }

    fn generate_music_like_signal(length: usize) -> Vec<f32> {
        let mut samples = vec![0.0; length];

        // Simulate music with chord (multiple pure tones)
        let frequencies = vec![261.63, 329.63, 392.00]; // C major chord

        for (i, sample) in samples.iter_mut().enumerate() {
            let t = i as f32 / SAMPLE_RATE as f32;
            let mut signal = 0.0;

            for &freq in &frequencies {
                signal += 0.15 * (2.0 * std::f32::consts::PI * freq * t).sin();
            }

            *sample = signal;
        }

        samples
    }

    fn calculate_snr(original: &AudioFrame, decoded: &AudioFrame) -> f32 {
        if original.samples.len() != decoded.samples.len() {
            return 0.0;
        }

        let mut signal_power = 0.0;
        let mut noise_power = 0.0;

        for (orig, dec) in original.samples.iter().zip(decoded.samples.iter()) {
            signal_power += orig * orig;
            let error = orig - dec;
            noise_power += error * error;
        }

        if noise_power == 0.0 {
            return 100.0; // Perfect reconstruction
        }

        10.0 * (signal_power / noise_power).log10()
    }
}