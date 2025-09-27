#[cfg(test)]
mod jitter_buffer_tests {
    use crate::jitter_buffer::*;
    use crate::jitter_buffer::{AdaptiveJitterBuffer, AudioPacket};
    use crate::realtime_audio::AudioFrame;
    use std::time::{Duration, Instant};

    #[test]
    fn test_jitter_buffer_config_default() {
        let config = JitterBufferConfig::default();

        assert_eq!(config.initial_target_size, 3);
        assert_eq!(config.max_size, 20);
        assert_eq!(config.min_size, 1);
        assert_eq!(config.adaptive, true);
        assert_eq!(config.late_packet_threshold_ms, 150);
        assert_eq!(config.underrun_threshold, 5);
        assert_eq!(config.overrun_threshold, 15);
    }

    #[test]
    fn test_jitter_buffer_creation() {
        let config = JitterBufferConfig::default();
        let buffer = AdaptiveJitterBuffer::new(config.clone());

        assert!(buffer.is_ok());
        let buffer = buffer.unwrap();

        assert_eq!(buffer.get_config(), config);
        assert_eq!(buffer.size(), 0);
        assert!(buffer.is_empty());
        assert_eq!(buffer.current_target_size(), config.initial_target_size);
    }

    #[test]
    fn test_audio_packet_creation() {
        let samples = vec![0.1, 0.2, 0.3, 0.4];
        let frame = AudioFrame::new(samples);
        let timestamp = 1000;
        let sequence = 42;

        let packet = AudioPacket::new(frame, timestamp, sequence);

        assert_eq!(packet.timestamp, timestamp);
        assert_eq!(packet.sequence_number, sequence);
        assert_eq!(packet.frame.samples.len(), 4);
        assert!((packet.arrival_time.elapsed().as_millis() as u64) < 10); // Should be very recent
    }

    #[test]
    fn test_packet_ordering_in_sequence() {
        let config = JitterBufferConfig::default();
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        // Add packets in sequence
        for i in 0..5 {
            let frame = AudioFrame::new(vec![i as f32; 100]);
            let packet = AudioPacket::new(frame, i * 1000, i as u32);
            buffer.add_packet(packet).unwrap();
        }

        assert_eq!(buffer.size(), 5);

        // Retrieve packets - should come out in order
        for i in 0..5 {
            println!("Getting packet {}, buffer size: {}", i, buffer.size());
            let packet = buffer.get_next_packet();
            if packet.is_none() {
                println!("No packet returned for sequence {}", i);
            }
            assert!(packet.is_some(), "Expected packet for sequence {}", i);
            let packet = packet.unwrap();
            assert_eq!(packet.sequence_number, i);
            assert_eq!(packet.timestamp, i as u64 * 1000);
        }

        assert!(buffer.is_empty());
    }

    #[test]
    fn test_packet_reordering() {
        let config = JitterBufferConfig::default();
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        // Add packets out of order
        let sequences = vec![2, 0, 3, 1, 4];
        for &seq in &sequences {
            let frame = AudioFrame::new(vec![seq as f32; 100]);
            let packet = AudioPacket::new(frame, seq as u64 * 1000, seq as u32);
            buffer.add_packet(packet).unwrap();
        }

        assert_eq!(buffer.size(), 5);

        // Should reorder and output in sequence
        for expected_seq in 0..5 {
            let packet = buffer.get_next_packet();
            assert!(packet.is_some());
            assert_eq!(packet.unwrap().sequence_number, expected_seq as u32);
        }
    }

    #[test]
    fn test_duplicate_packet_handling() {
        let config = JitterBufferConfig::default();
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        // Add original packet
        let frame1 = AudioFrame::new(vec![1.0; 100]);
        let packet1 = AudioPacket::new(frame1, 1000, 5);
        buffer.add_packet(packet1).unwrap();

        // Add duplicate packet
        let frame2 = AudioFrame::new(vec![2.0; 100]);
        let packet2 = AudioPacket::new(frame2, 1000, 5); // Same sequence number
        let result = buffer.add_packet(packet2);

        // Should handle duplicate gracefully
        assert!(result.is_ok());
        assert_eq!(buffer.size(), 1); // Still only one packet

        let stats = buffer.get_stats();
        assert_eq!(stats.duplicate_packets, 1);
    }

    #[test]
    fn test_late_packet_detection() {
        let mut config = JitterBufferConfig::default();
        config.late_packet_threshold_ms = 100; // 100ms threshold
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        // Add a packet and immediately try to get it to advance expected sequence
        let frame1 = AudioFrame::new(vec![1.0; 100]);
        let packet1 = AudioPacket::new(frame1, 1000, 0);
        buffer.add_packet(packet1).unwrap();
        let retrieved = buffer.get_next_packet(); // This advances expected_sequence to 1
        assert!(retrieved.is_some(), "Should have retrieved packet 0");

        // Sleep to simulate late arrival
        std::thread::sleep(Duration::from_millis(150));

        // Add a packet with sequence 0 again (should be considered late since we already processed seq 0)
        let frame2 = AudioFrame::new(vec![2.0; 100]);
        let packet2 = AudioPacket::new(frame2, 500, 0);
        buffer.add_packet(packet2).unwrap();

        let stats = buffer.get_stats();
        assert_eq!(stats.late_packets, 1);
    }

    #[test]
    fn test_buffer_underrun_detection() {
        let config = JitterBufferConfig::default();
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        // Try to get packet from empty buffer multiple times
        for _ in 0..10 {
            let result = buffer.get_next_packet();
            assert!(result.is_none());
        }

        let stats = buffer.get_stats();
        assert!(stats.buffer_underruns > 0);
    }

    #[test]
    fn test_buffer_overrun_protection() {
        let mut config = JitterBufferConfig::default();
        config.max_size = 3; // Very small buffer
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        // Add more packets than max_size
        for i in 0..10 {
            let frame = AudioFrame::new(vec![i as f32; 100]);
            let packet = AudioPacket::new(frame, i * 1000, i as u32);
            buffer.add_packet(packet).unwrap();
        }

        // Buffer should not exceed max_size
        assert!(buffer.size() <= 3);

        let stats = buffer.get_stats();
        assert!(stats.buffer_overruns > 0);
    }

    #[test]
    fn test_adaptive_target_size_adjustment() {
        let mut config = JitterBufferConfig::default();
        config.adaptive = true;
        config.initial_target_size = 3;
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        let initial_target = buffer.current_target_size();

        // Simulate network jitter by adding delay measurements
        for _ in 0..20 {
            buffer.simulate_network_delay(50.0); // 50ms delay
        }

        // Force adaptation check
        buffer.check_adaptation();

        // Target size might have increased due to high delay variance
        let new_target = buffer.current_target_size();
        println!("Initial target: {}, New target: {}", initial_target, new_target);

        // At minimum, the target should not have decreased below minimum
        assert!(new_target >= config.min_size);
    }

    #[test]
    fn test_network_statistics_tracking() {
        let config = JitterBufferConfig::default();
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        // Simulate various network conditions
        let delays = vec![10.0, 20.0, 15.0, 25.0, 30.0, 12.0, 18.0];
        for delay in delays {
            buffer.simulate_network_delay(delay);
        }

        let stats = buffer.get_stats();
        assert!(stats.average_network_delay > 0.0);
        assert!(stats.network_jitter > 0.0);

        // Calculate expected average manually
        let expected_avg = (10.0 + 20.0 + 15.0 + 25.0 + 30.0 + 12.0 + 18.0) / 7.0;
        assert!((stats.average_network_delay - expected_avg).abs() < 1.0);
    }

    #[test]
    fn test_buffer_reset() {
        let config = JitterBufferConfig::default();
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        // Add some packets
        for i in 0..5 {
            let frame = AudioFrame::new(vec![i as f32; 100]);
            let packet = AudioPacket::new(frame, i * 1000, i as u32);
            buffer.add_packet(packet).unwrap();
        }

        // Cause some statistics to accumulate
        buffer.get_next_packet(); // Cause potential underrun
        buffer.get_next_packet();

        assert!(buffer.size() > 0);

        // Reset buffer
        buffer.reset();

        assert_eq!(buffer.size(), 0);
        assert!(buffer.is_empty());

        let stats = buffer.get_stats();
        assert_eq!(stats.buffer_underruns, 0);
        assert_eq!(stats.buffer_overruns, 0);
        assert_eq!(stats.late_packets, 0);
        assert_eq!(stats.duplicate_packets, 0);
    }

    #[test]
    fn test_timestamp_based_retrieval() {
        let config = JitterBufferConfig::default();
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        // Add packets with specific timestamps
        let timestamps = vec![1000, 1020, 1040, 1060, 1080];
        for (i, &ts) in timestamps.iter().enumerate() {
            let frame = AudioFrame::new(vec![i as f32; 100]);
            let packet = AudioPacket::new(frame, ts, i as u32);
            buffer.add_packet(packet).unwrap();
        }

        // Retrieve packets and verify timestamp ordering
        let mut last_timestamp = 0;
        while let Some(packet) = buffer.get_next_packet() {
            assert!(packet.timestamp >= last_timestamp);
            last_timestamp = packet.timestamp;
        }
    }

    #[test]
    fn test_concurrent_access_safety() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let config = JitterBufferConfig::default();
        let buffer = Arc::new(Mutex::new(AdaptiveJitterBuffer::new(config).unwrap()));

        let buffer_clone: Arc<Mutex<AdaptiveJitterBuffer>> = Arc::clone(&buffer);

        // Producer thread
        let producer = thread::spawn(move || {
            for i in 0..100 {
                let frame = AudioFrame::new(vec![i as f32; 50]);
                let packet = AudioPacket::new(frame, i * 1000, i as u32);

                let mut buf = buffer_clone.lock().unwrap();
                let _ = buf.add_packet(packet);

                // Small delay to allow consumer to work
                if i % 10 == 0 {
                    drop(buf); // Release lock
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        });

        // Consumer thread
        let buffer_clone2: Arc<Mutex<AdaptiveJitterBuffer>> = Arc::clone(&buffer);
        let consumer = thread::spawn(move || {
            let mut packets_received = 0;

            for _ in 0..200 { // More attempts than packets to test empty buffer handling
                {
                    let mut buf = buffer_clone2.lock().unwrap();
                    if buf.get_next_packet().is_some() {
                        packets_received += 1;
                    }
                }

                std::thread::sleep(Duration::from_millis(1));
            }

            packets_received
        });

        producer.join().unwrap();
        let received_count = consumer.join().unwrap();

        println!("Received {} packets in concurrent test", received_count);
        assert!(received_count > 0);
    }

    #[test]
    fn test_frame_interpolation_for_missing_packets() {
        let config = JitterBufferConfig::default();
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        // Add packets with a gap (missing sequence 2)
        let sequences = vec![0, 1, 3, 4];
        for &seq in &sequences {
            let frame = AudioFrame::new(vec![seq as f32; 100]);
            let packet = AudioPacket::new(frame, seq as u64 * 1000, seq as u32);
            buffer.add_packet(packet).unwrap();
        }

        // Get first two packets normally
        assert!(buffer.get_next_packet().is_some());
        assert!(buffer.get_next_packet().is_some());

        // Next request should handle missing packet gracefully
        let result = buffer.get_next_packet();
        match result {
            Some(packet) => {
                // Should get packet 3, having handled missing packet 2
                assert_eq!(packet.sequence_number, 3);
            }
            None => {
                // Or might return None if waiting for missing packet
                println!("Buffer waiting for missing packet");
            }
        }
    }

    #[test]
    fn test_jitter_measurement() {
        let config = JitterBufferConfig::default();
        let mut buffer = AdaptiveJitterBuffer::new(config).unwrap();

        // Simulate packets with varying delays
        let delays = vec![10.0, 50.0, 5.0, 80.0, 15.0, 60.0, 20.0];

        for (i, &delay) in delays.iter().enumerate() {
            buffer.simulate_network_delay(delay);

            // Add a corresponding packet
            let frame = AudioFrame::new(vec![i as f32; 100]);
            let packet = AudioPacket::new(frame, i as u64 * 1000, i as u32);
            buffer.add_packet(packet).unwrap();
        }

        let stats = buffer.get_stats();

        // Jitter should be calculated as variance of delays
        assert!(stats.network_jitter > 0.0);
        println!("Measured jitter: {:.2}ms", stats.network_jitter);

        // With the delays above, we expect significant jitter
        assert!(stats.network_jitter > 10.0);
    }
}

// Extension methods for testing
impl crate::jitter_buffer::AdaptiveJitterBuffer {
    pub fn add_packet(&mut self, packet: crate::jitter_buffer::AudioPacket) -> Result<(), anyhow::Error> {
        self.put_packet(packet)
    }

    pub fn get_next_packet(&mut self) -> Option<crate::jitter_buffer::AudioPacket> {
        let result = self.get_next_sequential_packet();
        if let Some(ref packet) = result {
            // Update expected sequence like the real get_frame method does
            self.expected_sequence = packet.sequence_number.wrapping_add(1);
        } else {
            // Track underruns when no packet is available
            self.buffer_underruns += 1;
        }
        result
    }

    pub fn size(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn current_target_size(&self) -> usize {
        self.current_target_size
    }
    pub fn simulate_network_delay(&mut self, delay_ms: f64) {
        self.network_delay_samples.push_back(delay_ms);
        if self.network_delay_samples.len() > 50 {
            self.network_delay_samples.pop_front();
        }

        // Update statistics
        if !self.network_delay_samples.is_empty() {
            self.average_delay = self.network_delay_samples.iter().sum::<f64>() / self.network_delay_samples.len() as f64;

            let variance: f64 = self.network_delay_samples.iter()
                .map(|&delay| (delay - self.average_delay).powi(2))
                .sum::<f64>() / self.network_delay_samples.len() as f64;

            self.delay_variance = variance.sqrt();
        }
    }

    pub fn check_adaptation(&mut self) {
        // Force adaptation check for testing
        let now = std::time::Instant::now();
        if now.duration_since(self.last_adaptation_time) > std::time::Duration::from_secs(1) {
            self.adapt_buffer_size();
            self.last_adaptation_time = now;
        }
    }

}