use std::sync::{Arc, Mutex};
use std::io::{self, Write};
use crate::audio::AudioProcessor;
use crate::network::NetworkManager;
use crate::monitoring::HealthMonitor;
use anyhow::Result;

pub struct UserInterface {
    audio_processor: Arc<Mutex<AudioProcessor>>,
    network_manager: Arc<Mutex<NetworkManager>>,
    health_monitor: Option<Arc<HealthMonitor>>,
    connection_status: bool,
    input_level: f32,
    output_level: f32,
    running: bool,
}

impl UserInterface {
    pub fn new(
        audio_processor: Arc<Mutex<AudioProcessor>>,
        network_manager: Arc<Mutex<NetworkManager>>
    ) -> Self {
        Self {
            audio_processor,
            network_manager,
            health_monitor: None,
            connection_status: false,
            input_level: 0.0,
            output_level: 0.0,
            running: false,
        }
    }

    pub fn set_health_monitor(&mut self, health_monitor: Arc<HealthMonitor>) {
        self.health_monitor = Some(health_monitor);
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

    pub fn run_cli_interface(&mut self) -> Result<()> {
        println!("=== Humr Voice Communication ===");
        self.show_help();

        self.running = true;
        let mut input = String::new();

        while self.running {
            print!("humr> ");
            io::stdout().flush()?;

            input.clear();
            io::stdin().read_line(&mut input)?;

            let command = input.trim();
            if command.is_empty() {
                continue;
            }

            if let Err(e) = self.handle_command(command) {
                println!("Error: {}", e);
            }
        }

        Ok(())
    }

    fn show_help(&self) {
        println!("Commands:");
        println!("  connect <host:port> - Connect to remote peer");
        println!("  disconnect          - Disconnect from peer");
        println!("  volume <in> <out>   - Set input/output volume (0-100)");
        println!("  bitrate <rate>      - Set bit rate (8000-320000)");
        println!("  noise <on|off>      - Toggle noise cancellation");
        println!("  devices             - List available devices");
        println!("  status              - Show current status");
        println!("  health              - Show health report");
        println!("  metrics             - Show performance metrics");
        println!("  help                - Show this help");
        println!("  quit                - Exit application");
        println!();
    }

    fn handle_command(&mut self, command: &str) -> Result<()> {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(());
        }

        match parts[0] {
            "help" => self.show_help(),
            "quit" | "exit" => {
                self.running = false;
                println!("Goodbye!");
            },
            "status" => self.display_status(),
            "health" => self.show_health_report(),
            "metrics" => self.show_performance_metrics(),
            "devices" => self.list_devices(),
            "connect" => {
                if parts.len() < 2 {
                    println!("Usage: connect <host:port>");
                    return Ok(());
                }
                self.handle_connect_command(parts[1])?;
            },
            "disconnect" => self.handle_disconnect(),
            "volume" => {
                if parts.len() < 3 {
                    println!("Usage: volume <input> <output>");
                    return Ok(());
                }
                self.handle_volume_command(parts[1], parts[2])?;
            },
            "bitrate" => {
                if parts.len() < 2 {
                    println!("Usage: bitrate <rate>");
                    return Ok(());
                }
                self.handle_bitrate_command(parts[1])?;
            },
            "noise" => {
                if parts.len() < 2 {
                    println!("Usage: noise <on|off>");
                    return Ok(());
                }
                self.handle_noise_command(parts[1])?;
            },
            _ => println!("Unknown command: {}. Type 'help' for available commands.", parts[0]),
        }

        Ok(())
    }

    fn handle_connect_command(&mut self, host_port: &str) -> Result<()> {
        let parts: Vec<&str> = host_port.split(':').collect();
        if parts.len() != 2 {
            println!("Invalid format. Use: host:port");
            return Ok(());
        }

        let host = parts[0];
        let port: u16 = parts[1].parse().map_err(|_| anyhow::anyhow!("Invalid port number"))?;

        println!("Connecting to {}:{}...", host, port);
        self.show_connection_status(true);
        Ok(())
    }

    fn handle_disconnect(&mut self) {
        println!("Disconnecting...");
        self.show_connection_status(false);
    }

    fn handle_volume_command(&self, input_str: &str, output_str: &str) -> Result<()> {
        let input_vol: u8 = input_str.parse().map_err(|_| anyhow::anyhow!("Invalid input volume"))?;
        let output_vol: u8 = output_str.parse().map_err(|_| anyhow::anyhow!("Invalid output volume"))?;

        if input_vol > 100 || output_vol > 100 {
            return Err(anyhow::anyhow!("Volume must be between 0-100"));
        }

        self.update_volume_control(input_vol, output_vol);
        Ok(())
    }

    fn handle_bitrate_command(&self, rate_str: &str) -> Result<()> {
        let bitrate: u32 = rate_str.parse().map_err(|_| anyhow::anyhow!("Invalid bitrate"))?;

        if bitrate < 8000 || bitrate > 320000 {
            return Err(anyhow::anyhow!("Bitrate must be between 8000-320000"));
        }

        self.update_bit_rate_control(bitrate);
        Ok(())
    }

    fn handle_noise_command(&self, state: &str) -> Result<()> {
        match state.to_lowercase().as_str() {
            "on" | "true" | "1" => self.toggle_noise_cancellation(true),
            "off" | "false" | "0" => self.toggle_noise_cancellation(false),
            _ => return Err(anyhow::anyhow!("Use 'on' or 'off'")),
        }
        Ok(())
    }

    fn list_devices(&self) {
        println!("Available Devices:");
        println!("Input devices:");
        for device in self.get_available_devices(DeviceType::Input) {
            println!("  - {}", device);
        }
        println!("Output devices:");
        for device in self.get_available_devices(DeviceType::Output) {
            println!("  - {}", device);
        }
    }

    fn show_health_report(&self) {
        if let Some(ref monitor) = self.health_monitor {
            if let Some(report) = monitor.get_latest_report() {
                println!("Health Report:");
                println!("  Overall Status: {:?}", report.overall_status);
                println!("  Timestamp: {:?}", report.timestamp);
                println!("  Check Results:");
                for check in &report.checks {
                    println!("    {}: {:?}", check.name, check.status);
                    if !check.message.is_empty() {
                        println!("      {}", check.message);
                    }
                }
            } else {
                println!("No health report available. Run health checks first.");
            }
        } else {
            println!("Health monitoring not available.");
        }
    }

    fn show_performance_metrics(&self) {
        if let Some(ref monitor) = self.health_monitor {
            let metrics = monitor.get_metrics();
            println!("Performance Metrics:");
            println!("  CPU Usage: {:.1}%", metrics.cpu_usage_percent);
            println!("  Memory Usage: {:.1} MB", metrics.memory_usage_mb);
            println!("  Network Latency: {:.1} ms", metrics.network_latency_ms);
            println!("  Audio Latency: {:.1} ms", metrics.audio_processing_latency_ms);
            println!("  Packet Loss: {:.2}%", metrics.packet_loss_rate * 100.0);
        } else {
            println!("Performance monitoring not available.");
        }
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