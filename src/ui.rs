use std::sync::{Arc, Mutex};
use crate::audio::AudioProcessor;
use crate::network::NetworkManager;
use anyhow::Result;

pub struct UserInterface {
    audio_processor: Arc<Mutex<AudioProcessor>>,
    network_manager: Arc<Mutex<NetworkManager>>,
    connection_status: bool,
    input_level: f32,
    output_level: f32,
}

impl UserInterface {
    pub fn new(
        audio_processor: Arc<Mutex<AudioProcessor>>,
        network_manager: Arc<Mutex<NetworkManager>>
    ) -> Self {
        Self {
            audio_processor,
            network_manager,
            connection_status: false,
            input_level: 0.0,
            output_level: 0.0,
        }
    }

    pub fn get_available_devices(&self, device_type: DeviceType) -> Vec<String> {
        // THIS IS A STUB - Real implementation would enumerate platform audio devices
        // ASSUMPTION: Using placeholder devices for now
        match device_type {
            DeviceType::Input => vec!["default".to_string(), "microphone".to_string()],
            DeviceType::Output => vec!["default".to_string(), "speakers".to_string(), "headphones".to_string()],
        }
    }

    pub fn select_device(&self, device_type: DeviceType, device_name: &str) {
        if let Ok(mut processor) = self.audio_processor.lock() {
            match device_type {
                DeviceType::Input => {
                    processor.set_input_device(device_name);
                    println!("Selected input device: {}", device_name);
                }
                DeviceType::Output => {
                    processor.set_output_device(device_name);
                    println!("Selected output device: {}", device_name);
                }
            }
        }
    }

    pub fn update_volume_control(&self, input_vol: u8, output_vol: u8) {
        if let Ok(mut processor) = self.audio_processor.lock() {
            processor.set_input_volume(input_vol);
            processor.set_output_volume(output_vol);
            println!("Updated volumes - Input: {}%, Output: {}%", input_vol, output_vol);
        }
    }

    pub fn update_bit_rate_control(&self, bit_rate: u32) {
        if let Ok(mut processor) = self.audio_processor.lock() {
            processor.set_bit_rate(bit_rate);
            println!("Updated bit rate: {} bps", bit_rate);
        }
    }

    pub fn toggle_noise_cancellation(&self, enabled: bool) {
        if let Ok(mut processor) = self.audio_processor.lock() {
            processor.enable_noise_cancellation(enabled);
            println!("Noise cancellation: {}", if enabled { "ON" } else { "OFF" });
        }
    }

    pub fn show_connection_status(&mut self, connected: bool) {
        self.connection_status = connected;
        println!("Connection status: {}", if connected { "CONNECTED" } else { "DISCONNECTED" });
    }

    pub fn display_input_level(&mut self, level: f32) {
        self.input_level = level.max(-60.0).min(0.0);
        // THIS IS A STUB - Real implementation would show visual level meter
        if level > -20.0 {
            print!("Input: [████████████] ");
        } else if level > -40.0 {
            print!("Input: [██████      ] ");
        } else {
            print!("Input: [███         ] ");
        }
    }

    pub fn display_output_level(&mut self, level: f32) {
        self.output_level = level.max(-60.0).min(0.0);
        // THIS IS A STUB - Real implementation would show visual level meter
        if level > -20.0 {
            println!("Output: [████████████]");
        } else if level > -40.0 {
            println!("Output: [██████      ]");
        } else {
            println!("Output: [███         ]");
        }
    }

    // ASSUMPTION: Simple CLI interface for proof of concept
    pub fn run_cli_interface(&mut self) -> Result<()> {
        println!("=== Humr Voice Communication ===");
        println!("Commands:");
        println!("  connect <host:port> - Connect to remote peer");
        println!("  disconnect          - Disconnect from peer");
        println!("  volume <in> <out>   - Set input/output volume (0-100)");
        println!("  bitrate <rate>      - Set bit rate (8000-320000)");
        println!("  noise <on|off>      - Toggle noise cancellation");
        println!("  devices             - List available devices");
        println!("  quit                - Exit application");
        println!();

        // THIS IS A STUB - Real implementation would have proper CLI input loop
        // For now, just display current status
        self.display_status();
        Ok(())
    }

    fn display_status(&self) {
        println!("Current Status:");
        if let Ok(processor) = self.audio_processor.lock() {
            println!("  Input Device: {}", processor.input_device());
            println!("  Output Device: {}", processor.output_device());
            println!("  Bit Rate: {} bps", processor.bit_rate());
            println!("  Sample Rate: {} Hz", processor.sample_rate());
        }
        println!("  Connection: {}", if self.connection_status { "CONNECTED" } else { "DISCONNECTED" });
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Input,
    Output,
}