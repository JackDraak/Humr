use anyhow::Result;
use log::{info, warn, debug};
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use crate::realtime_audio::AudioFrame;

/// Adaptive jitter buffer configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JitterBufferConfig {
    /// Initial target buffer size (frames)
    pub initial_target_size: usize,
    /// Maximum buffer size (frames)
    pub max_size: usize,
    /// Minimum buffer size (frames)
    pub min_size: usize,
    /// Enable adaptive behavior
    pub adaptive: bool,
    /// Late packet threshold in milliseconds
    pub late_packet_threshold_ms: u32,
    /// Underrun threshold count
    pub underrun_threshold: u32,
    /// Overrun threshold count
    pub overrun_threshold: u32,
}

impl Default for JitterBufferConfig {
    fn default() -> Self {
        Self {
            initial_target_size: 3,
            max_size: 20,
            min_size: 1,
            adaptive: true,
            late_packet_threshold_ms: 150,
            underrun_threshold: 5,
            overrun_threshold: 15,
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

impl AudioPacket {
    pub fn new(frame: AudioFrame, timestamp: u64, sequence_number: u32) -> Self {
        Self {
            frame,
            sequence_number,
            timestamp,
            arrival_time: Instant::now(),
        }
    }
}

/// Adaptive jitter buffer for handling network timing variations
pub struct AdaptiveJitterBuffer {
    config: JitterBufferConfig,
    pub(crate) buffer: VecDeque<AudioPacket>,
    pub(crate) expected_sequence: u32,
    last_played_timestamp: u64,
    pub(crate) buffer_underruns: u64,
    pub(crate) buffer_overruns: u64,
    pub(crate) late_packets: u64,
    pub(crate) duplicate_packets: u64,

    // Adaptive parameters
    pub(crate) current_target_size: usize,
    pub(crate) network_delay_samples: VecDeque<f64>,
    pub(crate) last_adaptation_time: Instant,

    // Statistics
    pub(crate) average_delay: f64,
    pub(crate) delay_variance: f64,
}

impl AdaptiveJitterBuffer {
    /// Create new adaptive jitter buffer
    pub fn new(config: JitterBufferConfig) -> Result<Self> {
        info!("Creating adaptive jitter buffer with target size: {} frames", config.initial_target_size);

        Ok(Self {
            current_target_size: config.initial_target_size,
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
        })
    }

    /// Add incoming audio packet to buffer
    pub fn put_packet(&mut self, packet: AudioPacket) -> Result<()> {
        // Check for duplicate packets (packet already in buffer)
        for existing in &self.buffer {
            if existing.sequence_number == packet.sequence_number {
                self.duplicate_packets += 1;
                debug!("Dropping duplicate packet: seq={}", packet.sequence_number);
                return Ok(());
            }
        }

        // Calculate network delay for adaptation
        self.update_delay_statistics(&packet);

        // Check if packet is too late (either time-based or sequence-based)
        if self.is_packet_too_late(&packet) || packet.sequence_number < self.expected_sequence {
            self.late_packets += 1;
            debug!("Dropping late packet: seq={}, expected={}, delay={}ms",
                   packet.sequence_number, self.expected_sequence,
                   packet.arrival_time.elapsed().as_millis());
            return Ok(());
        }

        // Insert packet in correct position (sorted by sequence number)
        let insert_pos = self.find_insert_position(packet.sequence_number);
        self.buffer.insert(insert_pos, packet);

        // Prevent buffer overflow
        if self.buffer.len() > self.config.max_size {
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
    pub(crate) fn get_next_sequential_packet(&mut self) -> Option<AudioPacket> {
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
        let max_age = Duration::from_millis(self.config.late_packet_threshold_ms as u64);
        packet.arrival_time.elapsed() > max_age
    }

    /// Update network delay statistics for adaptation
    fn update_delay_statistics(&mut self, packet: &AudioPacket) {
        // Calculate packet delay (arrival time vs expected time)
        let _expected_arrival = Duration::from_millis(
            packet.sequence_number as u64 * 20
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
    pub(crate) fn adapt_buffer_size(&mut self) {
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
            (self.current_target_size + 2).min(self.config.max_size)
        } else if jitter < 10.0 && self.buffer_underruns == 0 {
            // Low jitter and no underruns: decrease buffer for lower latency
            (self.current_target_size - 1).max(self.config.min_size)
        } else {
            self.current_target_size
        };

        if new_target != self.current_target_size {
            info!("Adapting buffer size: {} -> {} (jitter: {:.1}ms)",
                  self.current_target_size, new_target, jitter);
            self.current_target_size = new_target;
        }
    }

    /// Get current configuration
    pub fn get_config(&self) -> JitterBufferConfig {
        self.config
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
            packets_received: self.buffer_underruns + self.buffer.len() as u64,
            average_network_delay: self.average_delay,
            network_jitter: self.delay_variance.sqrt(),
            buffer_underruns: self.buffer_underruns,
            buffer_overruns: self.buffer_overruns,
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
        self.current_target_size = self.config.initial_target_size;
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
    pub packets_received: u64,
    pub average_network_delay: f64,
    pub network_jitter: f64,
    pub buffer_underruns: u64,
    pub buffer_overruns: u64,
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