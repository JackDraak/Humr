use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use anyhow::Result;
use log::{info, warn, error};

use crate::audio::AudioProcessor;
use crate::realtime_audio::RealTimeAudioProcessor;
use crate::network::{NetworkManager, ConnectionConfig};
use crate::ui::UserInterface;
use crate::security::SecurityConfig;

pub struct VocalCommunicationApp {
    // Legacy audio processor (for configuration and UI)
    audio_processor: Arc<Mutex<AudioProcessor>>,
    // New real-time audio processor (lock-free)
    realtime_audio: Option<RealTimeAudioProcessor>,
    network_manager: Arc<Mutex<NetworkManager>>,
    user_interface: Arc<Mutex<UserInterface>>,
    is_running: bool,
}

impl VocalCommunicationApp {
    pub fn new() -> Self {
        // Initialize logging
        if env_logger::try_init().is_ok() {
            info!("Logging initialized");
        }

        let audio_processor = Arc::new(Mutex::new(AudioProcessor::new()));

        // Initialize real-time audio processor
        let realtime_audio = match RealTimeAudioProcessor::new() {
            Ok(processor) => {
                info!("Real-time audio processor created successfully");
                Some(processor)
            }
            Err(e) => {
                error!("Failed to create real-time audio processor: {}", e);
                None
            }
        };

        // Create security configuration for encrypted communications
        let security_config = SecurityConfig::new().expect("Failed to create security config");

        let network_manager = Arc::new(Mutex::new(NetworkManager::new(
            ConnectionConfig {
                remote_host: "localhost".to_string(),
                port: 8080,
                use_encryption: true,
                security_config: Some(security_config),
            }
        )));

        let user_interface = Arc::new(Mutex::new(UserInterface::new(
            audio_processor.clone(),
            network_manager.clone()
        )));

        Self {
            audio_processor,
            realtime_audio,
            network_manager,
            user_interface,
            is_running: false,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        info!("Starting Humr voice communication application");
        self.is_running = true;

        // Initialize and start real-time audio processor
        if let Some(ref mut realtime_audio) = self.realtime_audio {
            info!("Initializing real-time audio system");
            realtime_audio.initialize()?;
            realtime_audio.start()?;
            info!("Real-time audio system started successfully");
        } else {
            warn!("Real-time audio processor not available, running in compatibility mode");
            // Fall back to legacy threading model if real-time audio fails
            self.start_legacy_audio_threads();
        }

        // Start network processing thread (still using legacy approach for now)
        let network_clone = self.network_manager.clone();
        let running_flag = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let running_clone = running_flag.clone();

        thread::spawn(move || {
            Self::network_processing_loop(network_clone, running_clone);
        });

        // Start UI
        if let Ok(mut ui) = self.user_interface.lock() {
            ui.run_cli_interface()?;
        }

        Ok(())
    }

    /// Legacy audio threading for fallback compatibility
    fn start_legacy_audio_threads(&self) {
        warn!("Starting legacy audio threads (fallback mode)");

        let audio_clone = self.audio_processor.clone();
        let network_clone = self.network_manager.clone();
        let running_flag = Arc::new(std::sync::atomic::AtomicBool::new(true));
        let running_clone = running_flag.clone();

        thread::spawn(move || {
            Self::legacy_audio_capture_loop(audio_clone, network_clone, running_clone);
        });
    }

    fn legacy_audio_capture_loop(
        audio_processor: Arc<Mutex<AudioProcessor>>,
        network_manager: Arc<Mutex<NetworkManager>>,
        running: Arc<std::sync::atomic::AtomicBool>
    ) {
        warn!("Running legacy audio capture loop (fallback mode)");

        let mut frame_counter = 0;
        while running.load(std::sync::atomic::Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(20)); // ~50fps audio frames

            // LEGACY: Simulating audio capture for proof of concept
            let dummy_audio_frame = vec![0u8; 1024]; // 1KB frame

            if let (Ok(_processor), Ok(mut network)) =
                (audio_processor.lock(), network_manager.lock()) {

                // LEGACY STUB - Process audio frame
                // processor.process_audio_frame(&input_samples, &mut output_samples);

                // LEGACY STUB - Send to network
                if network.is_connected() {
                    let _ = network.send_audio_frame(&dummy_audio_frame);
                }
            }

            frame_counter += 1;
            if frame_counter % 250 == 0 { // Every ~5 seconds
                info!("Legacy audio capture running... (frame {})", frame_counter);
            }
        }
        warn!("Legacy audio capture loop stopped");
    }

    fn network_processing_loop(
        network_manager: Arc<Mutex<NetworkManager>>,
        running: Arc<std::sync::atomic::AtomicBool>
    ) {
        info!("Network processing loop started");

        while running.load(std::sync::atomic::Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(10));

            if let Ok(mut network) = network_manager.lock() {
                if network.is_connected() {
                    // THIS IS A STUB - Receive and process network audio
                    match network.receive_audio_frame() {
                        Ok(audio_data) => {
                            if !audio_data.is_empty() {
                                // THIS IS A STUB - Would queue for audio playback
                                // In Phase 2, this will integrate with the real-time audio processor
                                if log::log_enabled!(log::Level::Debug) {
                                    info!("Received audio frame: {} bytes", audio_data.len());
                                }
                            }
                        }
                        Err(e) => {
                            error!("Network receive error: {}", e);
                            break;
                        }
                    }
                }
            }
        }
        info!("Network processing loop stopped");
    }

    pub fn stop(&mut self) {
        info!("Stopping voice communication app...");
        self.is_running = false;

        // Stop real-time audio processor
        if let Some(ref mut realtime_audio) = self.realtime_audio {
            if let Err(e) = realtime_audio.stop() {
                error!("Error stopping real-time audio processor: {}", e);
            } else {
                info!("Real-time audio processor stopped successfully");
            }
        }

        info!("Voice communication app stopped");
    }

    /// Get real-time audio statistics for monitoring
    pub fn get_audio_stats(&self) -> Option<crate::realtime_audio::AudioStats> {
        self.realtime_audio.as_ref().map(|processor| processor.get_stats())
    }

    pub async fn connect_to_peer(&self, host: &str, port: u16) -> Result<()> {
        // Create new security config for this connection
        let security_config = SecurityConfig::new()?;

        let config = ConnectionConfig {
            remote_host: host.to_string(),
            port,
            use_encryption: true, // ASSUMPTION: Always use encryption for security
            security_config: Some(security_config),
        };

        if let Ok(mut network) = self.network_manager.lock() {
            network.update_config(config);
            network.establish_connection().await?;

            if let Ok(mut ui) = self.user_interface.lock() {
                ui.show_connection_status(true);
            }
        }

        Ok(())
    }

    pub fn disconnect_from_peer(&self) {
        if let Ok(mut network) = self.network_manager.lock() {
            network.disconnect();

            if let Ok(mut ui) = self.user_interface.lock() {
                ui.show_connection_status(false);
            }
        }
    }

    // ASSUMPTION: Helper methods for controlling the app
    pub fn set_volumes(&self, input: u8, output: u8) {
        if let Ok(ui) = self.user_interface.lock() {
            ui.update_volume_control(input, output);
        }
    }

    pub fn set_bit_rate(&self, bit_rate: u32) {
        if let Ok(ui) = self.user_interface.lock() {
            ui.update_bit_rate_control(bit_rate);
        }
    }

    pub fn toggle_noise_cancellation(&self, enabled: bool) {
        if let Ok(ui) = self.user_interface.lock() {
            ui.toggle_noise_cancellation(enabled);
        }
    }
}