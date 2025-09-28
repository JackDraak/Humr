use std::collections::HashMap;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::sync::{mpsc, Mutex};
use anyhow::{Result, anyhow};
use serde_json;

use crate::security::{SecureSession, SecureMessage, SecurityConfig};

pub struct NetworkManager {
    connection_config: ConnectionConfig,
    is_connected: bool,
    udp_socket: Option<Arc<UdpSocket>>,
    peer_addr: Option<SocketAddr>,
    // Async channels for UDP audio frames
    audio_tx: Option<mpsc::UnboundedSender<Vec<u8>>>,
    audio_rx: Option<mpsc::UnboundedReceiver<Vec<u8>>>,
    // Security components
    secure_session: Arc<Mutex<Option<SecureSession>>>,
    pending_handshake: bool,
}

#[derive(Clone)]
pub struct ConnectionConfig {
    pub remote_host: String,
    pub port: u16,
    pub use_encryption: bool,
    // Removed legacy encryption_key field - now handled by SecurityConfig
    pub security_config: Option<SecurityConfig>,
}

impl NetworkManager {
    pub fn new(config: ConnectionConfig) -> Self {
        // Initialize secure session if security config provided
        let secure_session = if let Some(security_config) = config.security_config.clone() {
            Arc::new(Mutex::new(Some(SecureSession::new(security_config))))
        } else {
            Arc::new(Mutex::new(None))
        };

        Self {
            connection_config: config,
            is_connected: false,
            udp_socket: None,
            peer_addr: None,
            audio_tx: None,
            audio_rx: None,
            secure_session,
            pending_handshake: false,
        }
    }

    /// Establish UDP connection with peer
    pub async fn establish_connection(&mut self) -> Result<()> {
        let local_addr = format!("0.0.0.0:{}", self.connection_config.port);
        let remote_addr = format!("{}:{}", self.connection_config.remote_host, self.connection_config.port);

        // Ensure we have security config if encryption is enabled
        {
            let session_guard = self.secure_session.lock().await;
            if self.connection_config.use_encryption && session_guard.is_none() {
                return Err(anyhow!("Encryption enabled but no security configuration provided"));
            }
        }

        // Bind UDP socket
        let socket = UdpSocket::bind(&local_addr).await
            .map_err(|e| anyhow!("Failed to bind UDP socket to {}: {}", local_addr, e))?;

        println!("UDP socket bound to {}", local_addr);

        // Parse remote address
        let peer_addr: SocketAddr = remote_addr.parse()
            .map_err(|e| anyhow!("Invalid remote address {}: {}", remote_addr, e))?;

        // Store socket and peer address
        let socket_arc = Arc::new(socket);
        self.udp_socket = Some(Arc::clone(&socket_arc));
        self.peer_addr = Some(peer_addr);

        // Set up audio channels
        let (audio_tx, audio_rx) = mpsc::unbounded_channel();
        self.audio_tx = Some(audio_tx);
        self.audio_rx = Some(audio_rx);

        // Start UDP receive loop
        self.start_udp_receiver(socket_arc, peer_addr).await?;

        // Perform handshake if encryption is enabled
        if self.connection_config.use_encryption {
            self.perform_udp_handshake(peer_addr).await?;
        }

        self.is_connected = true;
        println!("UDP connection established with {}", peer_addr);
        Ok(())
    }

    /// Start UDP receiver loop
    async fn start_udp_receiver(&mut self, socket: Arc<UdpSocket>, _peer_addr: SocketAddr) -> Result<()> {
        let audio_tx = self.audio_tx.as_ref()
            .ok_or_else(|| anyhow!("Audio transmitter not initialized"))?
            .clone();
        let secure_session = Arc::clone(&self.secure_session);

        tokio::spawn(async move {
            let mut buffer = vec![0u8; 2048]; // Smaller buffer for UDP packets

            loop {
                match socket.recv_from(&mut buffer).await {
                    Ok((len, _addr)) => {
                        let packet_data = buffer[..len].to_vec();

                        // Try to decrypt if we have a secure session
                        let audio_data = {
                            let session_guard = secure_session.lock().await;
                            if let Some(ref session) = *session_guard {
                                // Try to parse as SecureMessage
                                match serde_json::from_slice::<SecureMessage>(&packet_data) {
                                    Ok(secure_msg) => {
                                        match session.decrypt_audio_frame(secure_msg) {
                                            Ok(decrypted) => decrypted,
                                            Err(e) => {
                                                eprintln!("Decryption failed: {}", e);
                                                continue;
                                            }
                                        }
                                    }
                                    Err(_) => {
                                        // Assume plaintext for development/fallback
                                        packet_data
                                    }
                                }
                            } else {
                                // No encryption, use raw data
                                packet_data
                            }
                        };

                        if audio_tx.send(audio_data).is_err() {
                            eprintln!("Audio channel closed, stopping UDP receiver");
                            break;
                        }
                    }
                    Err(e) => {
                        eprintln!("UDP receive error: {}", e);
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Perform secure handshake over UDP
    async fn perform_udp_handshake(&mut self, peer_addr: SocketAddr) -> Result<()> {
        let socket = self.udp_socket.as_ref()
            .ok_or_else(|| anyhow!("No UDP socket available"))?;

        // We'll act as initiator for simplicity
        println!("Initiating secure UDP handshake with {}", peer_addr);

        let handshake_msg = {
            let mut session_guard = self.secure_session.lock().await;
            let session = session_guard.as_mut()
                .ok_or_else(|| anyhow!("No secure session available"))?;
            session.initiate_handshake()?
        };

        let handshake_json = serde_json::to_vec(&handshake_msg)?;

        // Send handshake packet
        socket.send_to(&handshake_json, peer_addr).await?;

        // Wait for response with timeout
        let mut buffer = vec![0u8; 4096];
        let response_result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            socket.recv_from(&mut buffer)
        ).await;

        match response_result {
            Ok(Ok((len, _))) => {
                let response_msg: SecureMessage = serde_json::from_slice(&buffer[..len])?;

                let mut session_guard = self.secure_session.lock().await;
                let session = session_guard.as_mut()
                    .ok_or_else(|| anyhow!("No secure session available"))?;
                session.process_handshake_response(response_msg)?;

                println!("Secure UDP handshake completed");
            }
            Ok(Err(e)) => return Err(anyhow!("UDP receive error during handshake: {}", e)),
            Err(_) => return Err(anyhow!("Handshake timeout - peer may not be ready")),
        }

        self.pending_handshake = false;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.is_connected = false;
        self.udp_socket = None;
        self.peer_addr = None;
        self.audio_tx = None;
        self.audio_rx = None;
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    /// Send audio frame over UDP
    pub async fn send_audio_frame(&self, frame_data: &[u8]) -> Result<()> {
        if !self.is_connected {
            return Err(anyhow!("Not connected"));
        }

        let socket = self.udp_socket.as_ref()
            .ok_or_else(|| anyhow!("No UDP socket available"))?;
        let peer_addr = self.peer_addr
            .ok_or_else(|| anyhow!("No peer address set"))?;

        // If encryption is enabled, encrypt the frame
        let data_to_send = if self.connection_config.use_encryption {
            let mut session_guard = self.secure_session.lock().await;
            if let Some(ref mut session) = *session_guard {
                if session.is_session_active() {
                    // Encrypt the audio frame
                    let encrypted_msg = session.encrypt_audio_frame(frame_data)?;
                    serde_json::to_vec(&encrypted_msg)?
                } else {
                    return Err(anyhow!("Secure session not established"));
                }
            } else {
                return Err(anyhow!("Encryption enabled but no secure session"));
            }
        } else {
            // Send plaintext
            frame_data.to_vec()
        };

        // Send UDP packet directly
        socket.send_to(&data_to_send, peer_addr).await
            .map_err(|e| anyhow!("Failed to send UDP packet: {}", e))?;

        Ok(())
    }

    pub fn receive_audio_frame(&mut self) -> Result<Vec<u8>> {
        if let Some(ref mut audio_rx) = self.audio_rx {
            match audio_rx.try_recv() {
                Ok(data) => Ok(data),
                Err(mpsc::error::TryRecvError::Empty) => Ok(vec![]),
                Err(mpsc::error::TryRecvError::Disconnected) => Err(anyhow!("Audio channel closed")),
            }
        } else {
            Err(anyhow!("No audio receiver available"))
        }
    }

    pub fn send_control_signal(&self, signal_type: &str, params: &HashMap<String, String>) -> Result<()> {
        // ASSUMPTION: Control signals would be JSON-encoded and sent with special prefix
        println!("Sending control signal: {} with params: {:?}", signal_type, params);
        Ok(())
    }

    pub async fn update_config(&mut self, config: ConnectionConfig) {
        // Update security session if config changed
        if let Some(security_config) = config.security_config.clone() {
            let mut session_guard = self.secure_session.lock().await;
            *session_guard = Some(SecureSession::new(security_config));
        }
        self.connection_config = config;
    }

    /// Get peer's verified identity (if secure session is active)
    pub async fn get_peer_identity(&self) -> Option<String> {
        let session_guard = self.secure_session.lock().await;
        session_guard.as_ref()
            .and_then(|session| session.get_peer_identity())
            .map(|pk| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, pk.as_bytes()))
    }

    /// Check if secure session is active
    pub async fn is_secure_session_active(&self) -> bool {
        let session_guard = self.secure_session.lock().await;
        session_guard.as_ref()
            .map(|session| session.is_session_active())
            .unwrap_or(false)
    }
}