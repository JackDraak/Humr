use anyhow::Result;
use log::{info, warn, debug};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use crate::realtime_audio::AudioFrame;

/// Adaptive jitter buffer configuration
#[derive(Debug, Clone)]
pub struct JitterBufferConfig {
    /// Minimum buffer size (frames)
    pub min_buffer_size: usize,
    /// Maximum buffer size (frames)
    pub max_buffer_size: usize,
    /// Target buffer size (frames)
    pub target_buffer_size: usize,
    /// Frame duration in milliseconds
    pub frame_duration_ms: u32,
    /// Maximum latency tolerance in milliseconds
    pub max_latency_ms: u32,
}

impl Default for JitterBufferConfig {
    fn default() -> Self {
        Self {
            min_buffer_size: 5,      // 100ms at 20ms frames
            max_buffer_size: 25,     // 500ms at 20ms frames
            target_buffer_size: 10,  // 200ms at 20ms frames
            frame_duration_ms: 20,   // 20ms frames
            max_latency_ms: 300,     // 300ms max acceptable latency
        }
    }
}

/// Network packet containing audio frame with timing information
#[derive(Debug, Clone)]
pub struct AudioPacket {
    pub frame: AudioFrame,
    pub sequence_number: u32,
    pub timestamp: u64,
    pub arrival_time: Instant,
}

/// Adaptive jitter buffer for handling network timing variations
pub struct AdaptiveJitterBuffer {
    config: JitterBufferConfig,
    buffer: VecDeque<AudioPacket>,
    expected_sequence: u32,
    last_played_timestamp: u64,
    buffer_underruns: u64,
    buffer_overruns: u64,
    late_packets: u64,
    duplicate_packets: u64,

    // Adaptive parameters
    current_target_size: usize,
    network_delay_samples: VecDeque<f64>,
    last_adaptation_time: Instant,

    // Statistics
    average_delay: f64,
    delay_variance: f64,
}

impl AdaptiveJitterBuffer {
    /// Create new adaptive jitter buffer
    pub fn new(config: JitterBufferConfig) -> Self {
        info!("Creating adaptive jitter buffer with target size: {} frames", config.target_buffer_size);

        Self {
            current_target_size: config.target_buffer_size,
            config,
            buffer: VecDeque::new(),
            expected_sequence: 0,
            last_played_timestamp: 0,
            buffer_underruns: 0,
            buffer_overruns: 0,
            late_packets: 0,
            duplicate_packets: 0,
            network_delay_samples: VecDeque::new(),
            last_adaptation_time: Instant::now(),
            average_delay: 0.0,
            delay_variance: 0.0,
        }
    }

    /// Add incoming audio packet to buffer
    pub fn put_packet(&mut self, packet: AudioPacket) -> Result<()> {
        // Check for duplicate packets
        if packet.sequence_number < self.expected_sequence {
            self.duplicate_packets += 1;
            debug!("Dropping duplicate packet: seq={}, expected={}",
                   packet.sequence_number, self.expected_sequence);
            return Ok(());
        }

        // Calculate network delay for adaptation
        self.update_delay_statistics(&packet);

        // Check if packet is too late
        if self.is_packet_too_late(&packet) {
            self.late_packets += 1;
            debug!("Dropping late packet: seq={}, delay={}ms",
                   packet.sequence_number,
                   packet.arrival_time.elapsed().as_millis());
            return Ok(());
        }

        // Insert packet in correct position (sorted by sequence number)
        let insert_pos = self.find_insert_position(packet.sequence_number);
        self.buffer.insert(insert_pos, packet);

        // Prevent buffer overflow
        if self.buffer.len() > self.config.max_buffer_size {
            self.buffer_overruns += 1;
            // Remove oldest packet
            if let Some(dropped) = self.buffer.pop_front() {
                warn!("Buffer overflow: dropped packet seq={}", dropped.sequence_number);
            }
        }

        // Adapt buffer size based on network conditions
        self.adapt_buffer_size();

        Ok(())
    }

    /// Get next audio frame for playback
    pub fn get_frame(&mut self) -> Option<AudioFrame> {
        // Check if we have enough packets buffered
        if self.buffer.len() < self.current_target_size && !self.buffer.is_empty() {
            // Not enough buffered yet, but don't starve if we have something
            if self.buffer.len() < 2 {
                debug!("Buffer underrun: only {} packets available", self.buffer.len());
                self.buffer_underruns += 1;
            }
        }

        // Get next sequential packet
        if let Some(packet) = self.get_next_sequential_packet() {
            self.last_played_timestamp = packet.timestamp;
            self.expected_sequence = packet.sequence_number.wrapping_add(1);
            return Some(packet.frame);
        }

        // No sequential packet available
        if !self.buffer.is_empty() {
            debug!("Gap in sequence: expected={}, available={:?}",
                   self.expected_sequence,
                   self.buffer.iter().take(3).map(|p| p.sequence_number).collect::<Vec<_>>());
        }

        self.buffer_underruns += 1;
        None
    }

    /// Find the correct position to insert a packet (maintaining sequence order)
    fn find_insert_position(&self, sequence: u32) -> usize {
        for (i, packet) in self.buffer.iter().enumerate() {
            if sequence <= packet.sequence_number {
                return i;
            }
        }
        self.buffer.len()
    }

    /// Get next sequential packet from buffer
    fn get_next_sequential_packet(&mut self) -> Option<AudioPacket> {
        // Look for expected sequence number
        for i in 0..self.buffer.len() {
            if self.buffer[i].sequence_number == self.expected_sequence {
                return self.buffer.remove(i);
            }
            // If we find a future packet, there's a gap
            if self.buffer[i].sequence_number > self.expected_sequence {
                break;
            }
        }

        // If buffer is getting full and we have a gap, skip to next available
        if self.buffer.len() > self.current_target_size * 2 {
            if let Some(packet) = self.buffer.pop_front() {
                debug!("Skipping gap: jumping from seq={} to seq={}",
                       self.expected_sequence, packet.sequence_number);
                self.expected_sequence = packet.sequence_number;
                return Some(packet);
            }
        }

        None
    }

    /// Check if packet arrived too late to be useful
    fn is_packet_too_late(&self, packet: &AudioPacket) -> bool {
        let max_age = Duration::from_millis(self.config.max_latency_ms as u64);
        packet.arrival_time.elapsed() > max_age
    }

    /// Update network delay statistics for adaptation
    fn update_delay_statistics(&mut self, packet: &AudioPacket) {
        // Calculate packet delay (arrival time vs expected time)
        let _expected_arrival = Duration::from_millis(
            packet.sequence_number as u64 * self.config.frame_duration_ms as u64
        );
        let actual_delay = packet.arrival_time.elapsed().as_millis() as f64;

        // Add to delay samples for variance calculation
        self.network_delay_samples.push_back(actual_delay);
        if self.network_delay_samples.len() > 50 {
            self.network_delay_samples.pop_front();
        }

        // Update running average and variance
        if !self.network_delay_samples.is_empty() {
            self.average_delay = self.network_delay_samples.iter().sum::<f64>()
                / self.network_delay_samples.len() as f64;

            let variance_sum: f64 = self.network_delay_samples.iter()
                .map(|delay| (delay - self.average_delay).powi(2))
                .sum();
            self.delay_variance = variance_sum / self.network_delay_samples.len() as f64;
        }
    }

    /// Adapt buffer size based on network conditions
    fn adapt_buffer_size(&mut self) {
        let now = Instant::now();

        // Only adapt every few seconds
        if now.duration_since(self.last_adaptation_time) < Duration::from_secs(2) {
            return;
        }

        self.last_adaptation_time = now;

        // Calculate adaptation based on network jitter
        let jitter = self.delay_variance.sqrt();

        let new_target = if jitter > 50.0 {
            // High jitter: increase buffer
            (self.current_target_size + 2).min(self.config.max_buffer_size)
        } else if jitter < 10.0 && self.buffer_underruns == 0 {
            // Low jitter and no underruns: decrease buffer for lower latency
            (self.current_target_size - 1).max(self.config.min_buffer_size)
        } else {
            self.current_target_size
        };

        if new_target != self.current_target_size {
            info!("Adapting buffer size: {} -> {} (jitter: {:.1}ms)",
                  self.current_target_size, new_target, jitter);
            self.current_target_size = new_target;
        }
    }

    /// Get buffer statistics
    pub fn get_stats(&self) -> JitterBufferStats {
        JitterBufferStats {
            current_size: self.buffer.len(),
            target_size: self.current_target_size,
            underruns: self.buffer_underruns,
            overruns: self.buffer_overruns,
            late_packets: self.late_packets,
            duplicate_packets: self.duplicate_packets,
            average_delay_ms: self.average_delay,
            delay_jitter_ms: self.delay_variance.sqrt(),
        }
    }

    /// Reset buffer state
    pub fn reset(&mut self) {
        info!("Resetting jitter buffer");
        self.buffer.clear();
        self.expected_sequence = 0;
        self.last_played_timestamp = 0;
        self.buffer_underruns = 0;
        self.buffer_overruns = 0;
        self.late_packets = 0;
        self.duplicate_packets = 0;
        self.network_delay_samples.clear();
        self.current_target_size = self.config.target_buffer_size;
        self.average_delay = 0.0;
        self.delay_variance = 0.0;
    }
}

/// Jitter buffer statistics
#[derive(Debug, Clone)]
pub struct JitterBufferStats {
    pub current_size: usize,
    pub target_size: usize,
    pub underruns: u64,
    pub overruns: u64,
    pub late_packets: u64,
    pub duplicate_packets: u64,
    pub average_delay_ms: f64,
    pub delay_jitter_ms: f64,
}

impl JitterBufferStats {
    pub fn packet_loss_rate(&self) -> f64 {
        let total_lost = self.underruns + self.late_packets;
        let total_expected = total_lost + self.current_size as u64;
        if total_expected > 0 {
            total_lost as f64 / total_expected as f64
        } else {
            0.0
        }
    }
}