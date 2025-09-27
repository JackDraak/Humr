use std::collections::HashMap;
use std::sync::mpsc;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use anyhow::{Result, anyhow};
use serde_json;

use crate::security::{SecureSession, SecureMessage, SecurityConfig};

pub struct NetworkManager {
    connection_config: ConnectionConfig,
    is_connected: bool,
    tx_channel: mpsc::Sender<Vec<u8>>,
    rx_channel: mpsc::Receiver<Vec<u8>>,
    // Security components
    secure_session: Option<SecureSession>,
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
        let (tx, rx) = mpsc::channel();

        // Initialize secure session if security config provided
        let secure_session = if let Some(security_config) = config.security_config.clone() {
            Some(SecureSession::new(security_config))
        } else {
            None
        };

        Self {
            connection_config: config,
            is_connected: false,
            tx_channel: tx,
            rx_channel: rx,
            secure_session,
            pending_handshake: false,
        }
    }

    /// Establish secure connection with peer authentication and key exchange
    pub async fn establish_connection(&mut self) -> Result<()> {
        let addr = format!("{}:{}", self.connection_config.remote_host, self.connection_config.port);

        // Ensure we have security config if encryption is enabled
        if self.connection_config.use_encryption && self.secure_session.is_none() {
            return Err(anyhow!("Encryption enabled but no security configuration provided"));
        }

        // ASSUMPTION: Try to connect as client first, fall back to server mode
        match TcpStream::connect(&addr).await {
            Ok(stream) => {
                println!("Connected as client to {}", addr);
                self.handle_connection(stream, true).await?;
            }
            Err(_) => {
                println!("Failed to connect as client, starting server on port {}", self.connection_config.port);
                self.start_server().await?;
            }
        }

        self.is_connected = true;
        Ok(())
    }

    async fn start_server(&mut self) -> Result<()> {
        let addr = format!("0.0.0.0:{}", self.connection_config.port);
        let listener = TcpListener::bind(&addr).await?;
        println!("Server listening on {}", addr);

        // ASSUMPTION: Accept only one connection for peer-to-peer voice chat
        let (stream, peer_addr) = listener.accept().await?;
        println!("Accepted connection from {}", peer_addr);

        self.handle_connection(stream, false).await?;
        Ok(())
    }

    /// Handle secure connection with optional handshake initiation
    async fn handle_connection(&mut self, mut stream: TcpStream, is_initiator: bool) -> Result<()> {
        // If encryption is enabled, perform secure handshake
        if self.connection_config.use_encryption {
            self.perform_secure_handshake(&mut stream, is_initiator).await?;
        }

        let tx = self.tx_channel.clone();
        let secure_session = self.secure_session.take(); // Move session into the task

        // Spawn connection handler task
        tokio::spawn(async move {
            let mut buffer = vec![0u8; 8192]; // Increased buffer for encrypted messages

            loop {
                match stream.read(&mut buffer).await {
                    Ok(0) => break, // Connection closed
                    Ok(n) => {
                        let message_data = buffer[..n].to_vec();

                        // If we have encryption, try to decrypt the message
                        let audio_data = if let Some(ref session) = secure_session {
                            // Parse as JSON message
                            match serde_json::from_slice::<SecureMessage>(&message_data) {
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
                                    // Might be plaintext - accept for now
                                    // ASSUMPTION: For graceful degradation during development
                                    message_data
                                }
                            }
                        } else {
                            // No encryption, use raw data
                            message_data
                        };

                        if tx.send(audio_data).is_err() {
                            break; // Channel closed
                        }
                    }
                    Err(_) => break, // Connection error
                }
            }
        });

        Ok(())
    }

    /// Perform secure handshake with peer
    async fn perform_secure_handshake(&mut self, stream: &mut TcpStream, is_initiator: bool) -> Result<()> {
        let session = self.secure_session.as_mut()
            .ok_or_else(|| anyhow!("No secure session available"))?;

        if is_initiator {
            // Initiate handshake
            println!("Initiating secure handshake...");
            let handshake_msg = session.initiate_handshake()?;
            let handshake_json = serde_json::to_vec(&handshake_msg)?;

            // Send handshake
            stream.write_all(&handshake_json).await?;

            // Wait for response
            let mut buffer = vec![0u8; 4096];
            let n = stream.read(&mut buffer).await?;
            let response_msg: SecureMessage = serde_json::from_slice(&buffer[..n])?;

            // Process response
            session.process_handshake_response(response_msg)?;
            println!("Secure handshake completed (initiator)");

        } else {
            // Wait for handshake
            println!("Waiting for secure handshake...");
            let mut buffer = vec![0u8; 4096];
            let n = stream.read(&mut buffer).await?;
            let handshake_msg: SecureMessage = serde_json::from_slice(&buffer[..n])?;

            // Process and respond
            if let Some(response) = session.process_handshake(handshake_msg)? {
                let response_json = serde_json::to_vec(&response)?;
                stream.write_all(&response_json).await?;
                println!("Secure handshake completed (responder)");
            }
        }

        self.pending_handshake = false;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        self.is_connected = false;
        // ASSUMPTION: Connection cleanup would happen here
    }

    pub fn is_connected(&self) -> bool {
        self.is_connected
    }

    /// Send encrypted audio frame to peer
    pub fn send_audio_frame(&mut self, frame_data: &[u8]) -> Result<()> {
        if !self.is_connected {
            return Err(anyhow!("Not connected"));
        }

        // If encryption is enabled, encrypt the frame
        let data_to_send = if self.connection_config.use_encryption {
            if let Some(ref mut session) = self.secure_session {
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

        // ASSUMPTION: For now, just put data in channel
        // Real implementation would send over TCP stream with proper framing
        self.tx_channel.send(data_to_send)?;
        Ok(())
    }

    pub fn receive_audio_frame(&self) -> Result<Vec<u8>> {
        match self.rx_channel.try_recv() {
            Ok(data) => Ok(data),
            Err(mpsc::TryRecvError::Empty) => Ok(vec![]),
            Err(mpsc::TryRecvError::Disconnected) => Err(anyhow::anyhow!("Connection closed")),
        }
    }

    pub fn send_control_signal(&self, signal_type: &str, params: &HashMap<String, String>) -> Result<()> {
        // ASSUMPTION: Control signals would be JSON-encoded and sent with special prefix
        println!("Sending control signal: {} with params: {:?}", signal_type, params);
        Ok(())
    }

    pub fn update_config(&mut self, config: ConnectionConfig) {
        // Update security session if config changed
        if let Some(security_config) = config.security_config.clone() {
            self.secure_session = Some(SecureSession::new(security_config));
        }
        self.connection_config = config;
    }

    /// Get peer's verified identity (if secure session is active)
    pub fn get_peer_identity(&self) -> Option<String> {
        self.secure_session.as_ref()
            .and_then(|session| session.get_peer_identity())
            .map(|pk| base64::Engine::encode(&base64::engine::general_purpose::STANDARD, pk.as_bytes()))
    }

    /// Check if secure session is active
    pub fn is_secure_session_active(&self) -> bool {
        self.secure_session.as_ref()
            .map(|session| session.is_session_active())
            .unwrap_or(false)
    }
}