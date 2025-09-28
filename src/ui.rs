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
        // Enhanced device enumeration using platform adapter
        use crate::platform::PlatformAudioAdapter;

        let adapter = PlatformAudioAdapter::new();
        match device_type {
            DeviceType::Input => {
                let mut devices = adapter.get_input_devices();
                if devices.is_empty() {
                    devices = vec!["default".to_string(), "microphone".to_string()];
                }
                devices
            },
            DeviceType::Output => {
                let mut devices = adapter.get_output_devices();
                if devices.is_empty() {
                    devices = vec!["default".to_string(), "speakers".to_string(), "headphones".to_string()];
                }
                devices
            },
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
        println!("╭─────────────────────────────────────────────────────────╮");
        println!("│                  Humr Voice Communication               │");
        println!("├─────────────────────────────────────────────────────────┤");
        println!("│ Connection Commands:                                    │");
        println!("│   connect <host:port>  Connect to remote peer          │");
        println!("│   disconnect           Disconnect from peer            │");
        println!("│                                                         │");
        println!("│ Audio Configuration:                                    │");
        println!("│   volume <in> <out>    Set volume levels (0-100)       │");
        println!("│   bitrate <rate>       Set bit rate (8000-320000)      │");
        println!("│   noise <on|off>       Toggle noise cancellation       │");
        println!("│   devices              List available audio devices     │");
        println!("│   device <type> <name> Select audio device             │");
        println!("│                                                         │");
        println!("│ Monitoring & Status:                                    │");
        println!("│   status               Show detailed system status      │");
        println!("│   health               Show health monitoring report    │");
        println!("│   metrics              Show performance metrics         │");
        println!("│   live                 Start live status monitoring     │");
        println!("│                                                         │");
        println!("│ System:                                                 │");
        println!("│   help                 Show this help                   │");
        println!("│   clear                Clear screen                     │");
        println!("│   quit                 Exit application                 │");
        println!("╰─────────────────────────────────────────────────────────╯");
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
            "clear" => {
                print!("\x1B[2J\x1B[1;1H"); // Clear screen and move cursor to top
                self.show_help();
            },
            "status" => self.display_enhanced_status(),
            "health" => self.show_health_report(),
            "metrics" => self.show_performance_metrics(),
            "devices" => self.list_devices(),
            "live" => self.start_live_monitoring(),
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
            "device" => {
                if parts.len() < 3 {
                    println!("Usage: device <input|output> <device_name>");
                    return Ok(());
                }
                self.handle_device_command(parts[1], parts[2])?;
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

    fn handle_device_command(&self, device_type: &str, device_name: &str) -> Result<()> {
        let device_type = match device_type.to_lowercase().as_str() {
            "input" | "in" => DeviceType::Input,
            "output" | "out" => DeviceType::Output,
            _ => return Err(anyhow::anyhow!("Device type must be 'input' or 'output'")),
        };

        self.select_device(device_type, device_name);
        Ok(())
    }

    fn display_enhanced_status(&self) {
        println!("╭─────────────────────────────────────────────────────────╮");
        println!("│                    System Status                        │");
        println!("├─────────────────────────────────────────────────────────┤");

        // Connection status
        let conn_status = if self.connection_status { "🟢 Connected" } else { "🔴 Disconnected" };
        println!("│ Connection:      {:<32} │", conn_status);

        // Audio status
        if let Ok(processor) = self.audio_processor.lock() {
            println!("│ Sample Rate:     {:<32} │", format!("{} Hz", processor.sample_rate()));
            println!("│ Bit Rate:        {:<32} │", format!("{} bps", processor.bit_rate()));
            println!("│ Input Volume:    {:<32} │", format!("{:.0}%", processor.get_input_gain() * 100.0));
            println!("│ Output Volume:   {:<32} │", format!("{}%", processor.get_output_volume()));
            println!("│ Noise Cancel:    {:<32} │", if processor.is_echo_cancellation_enabled() { "Enabled" } else { "Disabled" });
        }

        // Audio levels (simulated)
        println!("│ Input Level:     {:<32} │", format!("{:.1}%", self.input_level * 100.0));
        println!("│ Output Level:    {:<32} │", format!("{:.1}%", self.output_level * 100.0));

        println!("╰─────────────────────────────────────────────────────────╯");
        println!();
    }

    fn start_live_monitoring(&self) {
        println!("Starting live monitoring... (Press 'q' to quit)");
        println!("╭─────────────────────────────────────────────────────────╮");
        println!("│                   Live Status Monitor                   │");
        println!("├─────────────────────────────────────────────────────────┤");

        // Simple live display simulation (in a real implementation, this would run in a loop)
        for i in 0..5 {
            print!("\r│ Audio In:  ");
            let level = (i as f32 * 20.0) % 100.0;
            let bars = (level / 5.0) as usize;
            print!("{}", "█".repeat(bars));
            print!("{}", "░".repeat(20 - bars));
            print!(" {:.0}%", level);

            std::thread::sleep(std::time::Duration::from_millis(200));
        }

        println!("\n╰─────────────────────────────────────────────────────────╯");
        println!("Live monitoring stopped. Type 'live' to restart.");
    }

    fn list_devices(&self) {
        println!("╭─────────────────────────────────────────────────────────╮");
        println!("│                   Available Devices                     │");
        println!("├─────────────────────────────────────────────────────────┤");

        let input_devices = self.get_available_devices(DeviceType::Input);
        let output_devices = self.get_available_devices(DeviceType::Output);

        println!("│ Input Devices:                                          │");
        for (i, device) in input_devices.iter().enumerate() {
            println!("│   {}. {:<48} │", i + 1, device);
        }

        println!("│                                                         │");
        println!("│ Output Devices:                                         │");
        for (i, device) in output_devices.iter().enumerate() {
            println!("│   {}. {:<48} │", i + 1, device);
        }

        println!("├─────────────────────────────────────────────────────────┤");
        println!("│ Usage: device <input|output> <device_name>             │");
        println!("╰─────────────────────────────────────────────────────────╯");
        println!();
    }

    fn show_health_report(&self) {
        println!("╭─────────────────────────────────────────────────────────╮");
        println!("│                    Health Report                        │");
        println!("├─────────────────────────────────────────────────────────┤");

        if let Some(ref monitor) = self.health_monitor {
            if let Some(report) = monitor.get_latest_report() {
                let status_icon = match report.overall_status {
                    crate::monitoring::HealthStatus::Healthy => "🟢",
                    crate::monitoring::HealthStatus::Warning => "🟡",
                    crate::monitoring::HealthStatus::Critical => "🔴",
                    crate::monitoring::HealthStatus::Unknown => "⚪",
                };

                println!("│ Overall Status:  {} {:<28} │", status_icon, format!("{:?}", report.overall_status));
                println!("│ Report Time:     {:<32} │", format!("{:?}", report.timestamp));
                println!("│ Uptime:          {:<32} │", format!("{} seconds", report.uptime_seconds));
                println!("│                                                         │");
                println!("│ Component Health:                                       │");

                for check in &report.checks {
                    let check_icon = match check.status {
                        crate::monitoring::HealthStatus::Healthy => "✓",
                        crate::monitoring::HealthStatus::Warning => "⚠",
                        crate::monitoring::HealthStatus::Critical => "✗",
                        crate::monitoring::HealthStatus::Unknown => "?",
                    };

                    println!("│   {} {:<48} │", check_icon,
                        format!("{}: {:?}", check.name, check.status));

                    if !check.message.is_empty() {
                        println!("│     {:<50} │", check.message);
                    }
                }
            } else {
                println!("│ No health report available.                             │");
                println!("│ Health monitoring may still be initializing.           │");
            }
        } else {
            println!("│ Health monitoring not available.                        │");
        }

        println!("╰─────────────────────────────────────────────────────────╯");
        println!();
    }

    fn show_performance_metrics(&self) {
        println!("╭─────────────────────────────────────────────────────────╮");
        println!("│                  Performance Metrics                    │");
        println!("├─────────────────────────────────────────────────────────┤");

        if let Some(ref monitor) = self.health_monitor {
            let metrics = monitor.get_metrics();

            // System metrics
            println!("│ System Performance:                                     │");
            println!("│   CPU Usage:       {:<32} │", format!("{:.1}%", metrics.cpu_usage_percent));
            println!("│   Memory Usage:    {:<32} │", format!("{:.1} MB", metrics.memory_usage_mb));
            println!("│   Disk Usage:      {:<32} │", format!("{:.1}%", metrics.disk_usage_percent));
            println!("│                                                         │");

            // Audio metrics
            println!("│ Audio Performance:                                      │");
            println!("│   Audio Latency:   {:<32} │", format!("{:.1} ms", metrics.audio_processing_latency_ms));
            println!("│   Buffer Usage:    {:<32} │", format!("{:.1}%", metrics.audio_buffer_utilization * 100.0));
            println!("│   Dropouts/min:    {:<32} │", format!("{:.1}", metrics.audio_dropouts_per_minute));
            println!("│                                                         │");

            // Network metrics
            println!("│ Network Performance:                                    │");
            println!("│   Network Latency: {:<32} │", format!("{:.1} ms", metrics.network_latency_ms));
            println!("│   Packet Loss:     {:<32} │", format!("{:.2}%", metrics.packet_loss_rate * 100.0));
            println!("│   Bandwidth:       {:<32} │", format!("{:.1} Mbps", metrics.bandwidth_utilization_mbps));

            // Status indicators
            let cpu_status = if metrics.cpu_usage_percent > 80.0 { "🔴" }
                           else if metrics.cpu_usage_percent > 60.0 { "🟡" }
                           else { "🟢" };

            let audio_status = if metrics.audio_buffer_utilization > 0.9 { "🔴" }
                             else if metrics.audio_buffer_utilization > 0.7 { "🟡" }
                             else { "🟢" };

            let network_status = if metrics.packet_loss_rate > 0.05 { "🔴" }
                               else if metrics.packet_loss_rate > 0.01 { "🟡" }
                               else { "🟢" };

            println!("│                                                         │");
            println!("│ Status:                                                 │");
            println!("│   {} CPU      {} Audio      {} Network              │",
                     cpu_status, audio_status, network_status);

        } else {
            println!("│ Performance monitoring not available.                   │");
        }

        println!("╰─────────────────────────────────────────────────────────╯");
        println!();
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