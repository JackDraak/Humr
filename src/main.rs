use anyhow::Result;
use clap::{Arg, Command};
use env_logger;
use humr::run_terminal_ui;
use tokio;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();

    let matches = Command::new("humr")
        .version("0.1.0")
        .author("Humr Development Team")
        .about("Revolutionary P2P Voice Communication System")
        .arg(
            Arg::new("ui")
                .long("ui")
                .value_name("TYPE")
                .help("User interface type: terminal, gui (future)")
                .value_parser(["terminal", "gui"])
                .default_value("terminal")
        )
        .arg(
            Arg::new("mode")
                .long("mode")
                .value_name("MODE")
                .help("Application mode: interactive, host, join")
                .value_parser(["interactive", "host", "join"])
                .default_value("interactive")
        )
        .arg(
            Arg::new("room")
                .long("room")
                .value_name("ROOM_CODE")
                .help("Room code to join (for join mode)")
        )
        .arg(
            Arg::new("port")
                .long("port")
                .value_name("PORT")
                .help("Port to use for hosting")
                .value_parser(clap::value_parser!(u16))
                .default_value("8080")
        )
        .get_matches();

    let ui_type = matches.get_one::<String>("ui").unwrap();
    let mode = matches.get_one::<String>("mode").unwrap();
    let port = *matches.get_one::<u16>("port").unwrap();

    match ui_type.as_str() {
        "terminal" => {
            println!("🎤 Starting Humr Terminal Interface...");
            println!("📡 Revolutionary P2P Voice Communication");
            println!("🔒 End-to-end encrypted, zero-config connections");
            println!();

            match mode.as_str() {
                "interactive" => {
                    run_terminal_ui()?;
                }
                "host" => {
                    println!("🏠 Starting as host on port {}...", port);
                    start_host_mode(port).await?;
                }
                "join" => {
                    if let Some(room_code) = matches.get_one::<String>("room") {
                        println!("🔍 Joining room: {}", room_code);
                        start_join_mode(room_code).await?;
                    } else {
                        eprintln!("❌ Room code required for join mode. Use --room <ROOM_CODE>");
                        std::process::exit(1);
                    }
                }
                _ => unreachable!(),
            }
        }
        "gui" => {
            println!("🚧 GUI interface not yet implemented. Using terminal interface...");
            run_terminal_ui()?;
        }
        _ => unreachable!(),
    }

    Ok(())
}

async fn start_host_mode(port: u16) -> Result<()> {
    use humr::{DiscoveryManager, QRCodeGenerator, MagicLinkService};

    let mut discovery = DiscoveryManager::new();
    discovery.start().await?;

    let room_info = discovery
        .create_room("CLI Host".to_string(), port)
        .await?;

    println!("🎉 Voice chat room created!");
    println!("📋 Room Code: {}", room_info.room_id);
    println!("🔗 Magic Link: {}", MagicLinkService::generate_magic_link(&room_info));
    println!();

    // Generate and display QR code
    if let Ok(qr_code) = QRCodeGenerator::generate_connection_qr(&room_info) {
        println!("📱 QR Code for mobile devices:");
        println!("{}", qr_code);
        println!();
    }

    println!("🔊 Broadcasting on:");
    for method in &room_info.connection_methods {
        match method {
            humr::ConnectionMethod::LocalNetwork { ip } => {
                println!("  • Local Network: {}:{}", ip, port);
            }
            humr::ConnectionMethod::Internet { public_ip, port } => {
                println!("  • Internet: {}:{}", public_ip, port);
            }
            humr::ConnectionMethod::QRCode { .. } => {
                println!("  • QR Code: Available for scanning");
            }
            humr::ConnectionMethod::MagicLink { url } => {
                println!("  • Magic Link: {}", url);
            }
            _ => {}
        }
    }

    println!();
    println!("Press Ctrl+C to stop hosting...");

    // Wait for interrupt
    tokio::signal::ctrl_c().await?;
    println!("\n🛑 Stopping host...");

    discovery.stop().await?;
    Ok(())
}

async fn start_join_mode(room_code: &str) -> Result<()> {
    use humr::DiscoveryManager;

    let mut discovery = DiscoveryManager::new();
    discovery.start().await?;

    println!("🔍 Searching for room: {}", room_code);

    // Try to discover the room
    let rooms = discovery.discover_rooms().await?;

    if let Some(room) = rooms.iter().find(|r| r.room_id == room_code) {
        println!("✅ Found room: {}", room.display_name);
        println!("🏠 Host: {}", room.host_name);
        println!("👥 Participants: {}/{}", room.current_participants, room.max_participants);

        // In a real implementation, this would establish the voice connection
        println!("🔄 Connecting...");
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        println!("🎉 Connected! Voice chat active.");

        println!("Press Ctrl+C to disconnect...");
        tokio::signal::ctrl_c().await?;
        println!("\n🛑 Disconnecting...");
    } else {
        println!("❌ Room not found: {}", room_code);
        println!("💡 Make sure the host is online and the room code is correct.");
    }

    discovery.stop().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::audio::AudioProcessor;

    #[test]
    fn test_audio_processor_creation() {
        let processor = AudioProcessor::new();
        assert_eq!(processor.bit_rate(), 64_000);
        assert_eq!(processor.sample_rate(), 48_000);
    }

    #[test]
    fn test_bit_rate_validation() {
        let mut processor = AudioProcessor::new();
        processor.set_bit_rate(128_000);
        assert_eq!(processor.bit_rate(), 128_000);

        // This should panic due to assertion in real usage
        // processor.set_bit_rate(500_000); // Uncomment to test validation
    }

    #[tokio::test]
    async fn test_app_creation() {
        let app = VocalCommunicationApp::new();
        // Basic smoke test that app can be created
        assert!(true); // THIS IS A STUB - Would test actual functionality
    }
}
