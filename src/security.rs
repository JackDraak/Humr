use anyhow::{Result, anyhow};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, aead::{Aead, AeadCore, KeyInit}};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, SharedSecret};
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;
use serde::{Serialize, Deserialize};
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};

/// Security configuration for the voice communication system
#[derive(Clone)]
pub struct SecurityConfig {
    /// Long-term identity signing key for authentication
    pub identity_signing_key: SigningKey,
    /// Our public verifying key
    pub identity_verifying_key: VerifyingKey,
    /// Trusted public keys (contacts/peers)
    pub trusted_keys: Vec<VerifyingKey>,
    /// Current session encryption cipher
    pub session_cipher: Option<ChaCha20Poly1305>,
    /// Enable/disable security features
    pub encryption_enabled: bool,
    pub authentication_required: bool,
}

impl SecurityConfig {
    /// Create new security configuration with fresh identity
    pub fn new() -> Result<Self> {
        let identity_signing_key = SigningKey::from_bytes(&rand::random::<[u8; 32]>());
        let identity_verifying_key = identity_signing_key.verifying_key();

        Ok(Self {
            identity_signing_key,
            identity_verifying_key,
            trusted_keys: Vec::new(),
            session_cipher: None,
            encryption_enabled: true,
            authentication_required: true,
        })
    }

    /// Load security config from existing identity key
    pub fn from_identity_key(secret_key_bytes: &[u8]) -> Result<Self> {
        if secret_key_bytes.len() != 32 {
            return Err(anyhow!("Invalid secret key length"));
        }

        let secret_key_array: [u8; 32] = secret_key_bytes.try_into()
            .map_err(|_| anyhow!("Invalid secret key length"))?;
        let identity_signing_key = SigningKey::from_bytes(&secret_key_array);
        let identity_verifying_key = identity_signing_key.verifying_key();

        Ok(Self {
            identity_signing_key,
            identity_verifying_key,
            trusted_keys: Vec::new(),
            session_cipher: None,
            encryption_enabled: true,
            authentication_required: true,
        })
    }

    /// Add a trusted peer's public key
    pub fn add_trusted_peer(&mut self, public_key: VerifyingKey) {
        if !self.trusted_keys.contains(&public_key) {
            self.trusted_keys.push(public_key);
        }
    }

    /// Check if a public key is trusted
    pub fn is_peer_trusted(&self, public_key: &VerifyingKey) -> bool {
        self.trusted_keys.contains(public_key)
    }

    /// Get our public identity for sharing with peers
    pub fn get_public_identity(&self) -> VerifyingKey {
        self.identity_verifying_key
    }

    /// Export identity keypair for backup/storage
    pub fn export_identity(&self) -> String {
        BASE64.encode(self.identity_signing_key.to_bytes())
    }
}

/// Message types for secure communication protocol
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
pub enum SecureMessage {
    /// Initial handshake with identity and ephemeral key
    Handshake {
        identity_public_key: String, // base64 encoded
        ephemeral_public_key: String, // base64 encoded x25519
        signature: String, // base64 encoded signature
        timestamp: u64,
    },
    /// Handshake response with peer's ephemeral key
    HandshakeResponse {
        ephemeral_public_key: String,
        signature: String,
        timestamp: u64,
    },
    /// Encrypted audio frame
    EncryptedAudio {
        nonce: String, // base64 encoded
        ciphertext: String, // base64 encoded
        frame_number: u64,
    },
    /// Connection termination
    Disconnect {
        reason: String,
        signature: String,
    },
}

/// Secure session manager for end-to-end encrypted communication
pub struct SecureSession {
    config: SecurityConfig,
    peer_identity: Option<VerifyingKey>,
    shared_secret: Option<SharedSecret>,
    ephemeral_secret: Option<EphemeralSecret>,
    session_key: Option<[u8; 32]>,
    frame_counter: u64,
    is_initiator: bool,
}

impl SecureSession {
    /// Create new secure session
    pub fn new(config: SecurityConfig) -> Self {
        Self {
            config,
            peer_identity: None,
            shared_secret: None,
            ephemeral_secret: None,
            session_key: None,
            frame_counter: 0,
            is_initiator: false,
        }
    }

    /// Initiate secure handshake with a peer
    pub fn initiate_handshake(&mut self) -> Result<SecureMessage> {
        self.is_initiator = true;

        // Generate ephemeral keypair for this session
        let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
        let ephemeral_public = X25519PublicKey::from(&ephemeral_secret);

        // Create handshake message
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        // Sign the ephemeral public key to prove identity
        let mut hasher = Sha256::new();
        hasher.update(ephemeral_public.as_bytes());
        hasher.update(timestamp.to_be_bytes());
        let hash = hasher.finalize();

        let signature = self.config.identity_signing_key.sign(&hash);

        self.ephemeral_secret = Some(ephemeral_secret);

        Ok(SecureMessage::Handshake {
            identity_public_key: BASE64.encode(self.config.identity_verifying_key.as_bytes()),
            ephemeral_public_key: BASE64.encode(ephemeral_public.as_bytes()),
            signature: BASE64.encode(signature.to_bytes()),
            timestamp,
        })
    }

    /// Process incoming handshake and generate response
    pub fn process_handshake(&mut self, handshake: SecureMessage) -> Result<Option<SecureMessage>> {
        match handshake {
            SecureMessage::Handshake {
                identity_public_key,
                ephemeral_public_key,
                signature,
                timestamp,
            } => {
                // Verify timestamp is recent (within 5 minutes)
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();
                if current_time.saturating_sub(timestamp) > 300 {
                    return Err(anyhow!("Handshake timestamp too old"));
                }

                // Parse peer's identity
                let peer_identity_bytes = BASE64.decode(identity_public_key)?;
                let peer_identity_array: [u8; 32] = peer_identity_bytes.try_into()
                    .map_err(|_| anyhow!("Invalid public key length"))?;
                let peer_identity = VerifyingKey::from_bytes(&peer_identity_array)
                    .map_err(|e| anyhow!("Invalid public key: {}", e))?;

                // ASSUMPTION: In production, you might want configurable trust models
                // For now, we'll add any connecting peer to trusted list automatically
                // Real implementation should require manual verification or PKI
                if !self.config.is_peer_trusted(&peer_identity) {
                    println!("WARNING: Peer {} not in trusted list, adding automatically",
                             BASE64.encode(peer_identity.as_bytes()));
                    // In production: return Err(anyhow!("Untrusted peer"));
                }

                // Parse ephemeral key
                let peer_ephemeral_bytes = BASE64.decode(ephemeral_public_key)?;
                let peer_ephemeral_array: [u8; 32] = peer_ephemeral_bytes.try_into()
                    .map_err(|_| anyhow!("Invalid ephemeral key length"))?;
                let peer_ephemeral = X25519PublicKey::from(peer_ephemeral_array);

                // Verify signature
                let signature_bytes = BASE64.decode(signature)?;
                let signature_array: [u8; 64] = signature_bytes.try_into()
                    .map_err(|_| anyhow!("Invalid signature length"))?;
                let signature = Signature::from_bytes(&signature_array);

                let mut hasher = Sha256::new();
                hasher.update(peer_ephemeral.as_bytes());
                hasher.update(timestamp.to_be_bytes());
                let hash = hasher.finalize();

                peer_identity.verify(&hash, &signature)
                    .map_err(|e| anyhow!("Signature verification failed: {}", e))?;

                // Generate our ephemeral keypair and shared secret
                let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
                let ephemeral_public = X25519PublicKey::from(&ephemeral_secret);
                let shared_secret = ephemeral_secret.diffie_hellman(&peer_ephemeral);

                // Derive session key using HKDF-like approach
                let session_key = self.derive_session_key(&shared_secret, &peer_identity)?;

                // Store session state
                self.peer_identity = Some(peer_identity);
                self.shared_secret = Some(shared_secret);
                self.session_key = Some(session_key);

                // Create cipher for this session
                let key = Key::from_slice(&session_key);
                self.config.session_cipher = Some(ChaCha20Poly1305::new(key));

                // Generate response
                let response_timestamp = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();

                let mut response_hasher = Sha256::new();
                response_hasher.update(ephemeral_public.as_bytes());
                response_hasher.update(response_timestamp.to_be_bytes());
                let response_hash = response_hasher.finalize();

                let response_signature = self.config.identity_signing_key.sign(&response_hash);

                Ok(Some(SecureMessage::HandshakeResponse {
                    ephemeral_public_key: BASE64.encode(ephemeral_public.as_bytes()),
                    signature: BASE64.encode(response_signature.to_bytes()),
                    timestamp: response_timestamp,
                }))
            }
            _ => Err(anyhow!("Expected handshake message")),
        }
    }

    /// Process handshake response (for initiator)
    pub fn process_handshake_response(&mut self, response: SecureMessage) -> Result<()> {
        match response {
            SecureMessage::HandshakeResponse {
                ephemeral_public_key,
                signature,
                timestamp,
            } => {
                let peer_identity = self.peer_identity
                    .ok_or_else(|| anyhow!("No peer identity established"))?;

                // Verify timestamp
                let current_time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs();
                if current_time.saturating_sub(timestamp) > 300 {
                    return Err(anyhow!("Response timestamp too old"));
                }

                // Parse peer's ephemeral key
                let peer_ephemeral_bytes = BASE64.decode(ephemeral_public_key)?;
                let peer_ephemeral_array: [u8; 32] = peer_ephemeral_bytes.try_into()
                    .map_err(|_| anyhow!("Invalid ephemeral key length"))?;
                let peer_ephemeral = X25519PublicKey::from(peer_ephemeral_array);

                // Verify signature
                let signature_bytes = BASE64.decode(signature)?;
                let signature_array: [u8; 64] = signature_bytes.try_into()
                    .map_err(|_| anyhow!("Invalid signature length"))?;
                let signature = Signature::from_bytes(&signature_array);

                let mut hasher = Sha256::new();
                hasher.update(peer_ephemeral.as_bytes());
                hasher.update(timestamp.to_be_bytes());
                let hash = hasher.finalize();

                peer_identity.verify(&hash, &signature)
                    .map_err(|e| anyhow!("Signature verification failed: {}", e))?;

                // Complete DH exchange
                let ephemeral_secret = self.ephemeral_secret.take()
                    .ok_or_else(|| anyhow!("No ephemeral secret"))?;
                let shared_secret = ephemeral_secret.diffie_hellman(&peer_ephemeral);

                // Derive session key
                let session_key = self.derive_session_key(&shared_secret, &peer_identity)?;

                // Store session state
                self.shared_secret = Some(shared_secret);
                self.session_key = Some(session_key);

                // Create cipher
                let key = Key::from_slice(&session_key);
                self.config.session_cipher = Some(ChaCha20Poly1305::new(key));

                println!("Secure session established with peer");
                Ok(())
            }
            _ => Err(anyhow!("Expected handshake response")),
        }
    }

    /// Encrypt audio frame for transmission
    pub fn encrypt_audio_frame(&mut self, audio_data: &[u8]) -> Result<SecureMessage> {
        let cipher = self.config.session_cipher.as_ref()
            .ok_or_else(|| anyhow!("No active session"))?;

        // Generate unique nonce for this frame
        let nonce = ChaCha20Poly1305::generate_nonce(&mut OsRng);

        // Encrypt the audio data
        let ciphertext = cipher.encrypt(&nonce, audio_data)
            .map_err(|_| anyhow!("Encryption failed"))?;

        self.frame_counter += 1;

        Ok(SecureMessage::EncryptedAudio {
            nonce: BASE64.encode(nonce.as_slice()),
            ciphertext: BASE64.encode(ciphertext),
            frame_number: self.frame_counter,
        })
    }

    /// Decrypt received audio frame
    pub fn decrypt_audio_frame(&self, message: SecureMessage) -> Result<Vec<u8>> {
        match message {
            SecureMessage::EncryptedAudio { nonce, ciphertext, frame_number: _ } => {
                let cipher = self.config.session_cipher.as_ref()
                    .ok_or_else(|| anyhow!("No active session"))?;

                let nonce_bytes = BASE64.decode(nonce)?;
                let nonce = Nonce::from_slice(&nonce_bytes);

                let ciphertext_bytes = BASE64.decode(ciphertext)?;

                let plaintext = cipher.decrypt(nonce, ciphertext_bytes.as_slice())
                    .map_err(|_| anyhow!("Decryption failed"))?;

                Ok(plaintext)
            }
            _ => Err(anyhow!("Expected encrypted audio message")),
        }
    }

    /// Derive session key from shared secret and peer identity
    fn derive_session_key(&self, shared_secret: &SharedSecret, peer_identity: &VerifyingKey) -> Result<[u8; 32]> {
        // HKDF-like key derivation
        let mut hasher = Sha256::new();
        hasher.update(shared_secret.as_bytes());
        hasher.update(self.config.identity_verifying_key.as_bytes());
        hasher.update(peer_identity.as_bytes());
        hasher.update(b"HUMR_SESSION_KEY_V1");

        let hash = hasher.finalize();
        Ok(hash.into())
    }

    /// Check if session is active and secure
    pub fn is_session_active(&self) -> bool {
        self.config.session_cipher.is_some() && self.peer_identity.is_some()
    }

    /// Get peer's verified identity
    pub fn get_peer_identity(&self) -> Option<&VerifyingKey> {
        self.peer_identity.as_ref()
    }
}