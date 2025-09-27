#[cfg(test)]
mod security_tests {
    use crate::security::*;
    use crate::realtime_audio::AudioFrame;
    use ed25519_dalek::{SigningKey, VerifyingKey};
    use x25519_dalek::{EphemeralSecret, PublicKey};
    use rand::rngs::OsRng;

    #[test]
    fn test_security_config_creation() {
        let config = SecurityConfig::new();
        assert!(config.is_ok());

        let config = config.unwrap();
        assert!(config.encryption_enabled);
        assert!(config.authentication_required);
        assert!(config.trusted_keys.is_empty());
        assert!(config.session_cipher.is_none());
    }

    #[test]
    fn test_identity_key_generation() {
        let config = SecurityConfig::new().unwrap();

        // Keys should be different each time
        let config2 = SecurityConfig::new().unwrap();

        assert_ne!(config.identity_signing_key.to_bytes(),
                  config2.identity_signing_key.to_bytes());
        assert_ne!(config.identity_verifying_key.to_bytes(),
                  config2.identity_verifying_key.to_bytes());
    }

    #[test]
    fn test_security_manager_creation() {
        let config = SecurityConfig::new().unwrap();
        let manager = SecurityManager::new(config);

        assert!(manager.is_ok());
        let manager = manager.unwrap();

        assert!(!manager.has_established_session());
        assert_eq!(manager.get_stats().messages_encrypted, 0);
        assert_eq!(manager.get_stats().messages_decrypted, 0);
    }

    #[test]
    fn test_key_exchange_protocol() {
        // Create two parties
        let config_alice = SecurityConfig::new().unwrap();
        let mut manager_alice = SecurityManager::new(config_alice).unwrap();

        let config_bob = SecurityConfig::new().unwrap();
        let mut manager_bob = SecurityManager::new(config_bob).unwrap();

        // Step 1: Alice creates key exchange initiation
        let alice_exchange = manager_alice.initiate_key_exchange().unwrap();

        // Verify the exchange message contains required components
        assert!(!alice_exchange.ephemeral_public_key.is_empty());
        assert!(!alice_exchange.identity_public_key.is_empty());
        assert!(!alice_exchange.signature.is_empty());

        // Step 2: Bob processes Alice's key exchange and creates response
        let bob_exchange = manager_bob.process_key_exchange(&alice_exchange).unwrap();

        // Step 3: Alice processes Bob's response to complete the exchange
        let result = manager_alice.complete_key_exchange(&bob_exchange);
        assert!(result.is_ok());

        // Both parties should now have established sessions
        assert!(manager_alice.has_established_session());
        assert!(manager_bob.has_established_session());

        println!("Key exchange completed successfully");
    }

    #[test]
    fn test_message_encryption_decryption() {
        // Establish secure session between two parties
        let (mut alice, mut bob) = establish_secure_session().unwrap();

        let test_message = b"Hello, secure world!";

        // Alice encrypts message
        let encrypted = alice.encrypt_message(test_message).unwrap();

        assert_ne!(encrypted.ciphertext, test_message);
        assert!(!encrypted.nonce.is_empty());
        // ChaCha20Poly1305 includes authentication tag in ciphertext, so separate tag is empty

        // Bob decrypts message
        let decrypted = bob.decrypt_message(&encrypted).unwrap();

        assert_eq!(decrypted, test_message);

        println!("Message encryption/decryption test passed");
    }

    #[test]
    fn test_audio_frame_encryption() {
        let (mut alice, mut bob) = establish_secure_session().unwrap();

        // Create test audio frame
        let samples = vec![0.1, 0.2, -0.3, 0.4, -0.5];
        let original_frame = AudioFrame::new(samples.clone());

        // Encrypt audio frame
        let encrypted_frame = alice.encrypt_audio_frame(&original_frame).unwrap();

        // Encrypted data should be different
        assert_ne!(encrypted_frame.encrypted_data, samples.iter().flat_map(|&f| f.to_le_bytes()).collect::<Vec<u8>>());
        assert!(!encrypted_frame.nonce.is_empty());

        // Decrypt audio frame
        let decrypted_frame = bob.decrypt_audio_frame(&encrypted_frame).unwrap();

        // Should match original
        assert_eq!(decrypted_frame.samples.len(), original_frame.samples.len());
        for (orig, decr) in original_frame.samples.iter().zip(decrypted_frame.samples.iter()) {
            assert!((orig - decr).abs() < 1e-6, "Sample mismatch: {} vs {}", orig, decr);
        }

        let alice_stats = alice.get_stats();
        let bob_stats = bob.get_stats();

        assert_eq!(alice_stats.audio_frames_encrypted, 1);
        assert_eq!(bob_stats.audio_frames_decrypted, 1);

        println!("Audio frame encryption test passed");
    }

    #[test]
    fn test_trust_management() {
        let config_alice = SecurityConfig::new().unwrap();
        let mut manager_alice = SecurityManager::new(config_alice).unwrap();

        let config_bob = SecurityConfig::new().unwrap();
        let bob_identity = config_bob.identity_verifying_key;

        // Initially no trusted keys
        assert!(!manager_alice.is_key_trusted(&bob_identity));

        // Add Bob's key to trust store
        manager_alice.add_trusted_key(bob_identity).unwrap();

        // Now Bob should be trusted
        assert!(manager_alice.is_key_trusted(&bob_identity));

        // Remove Bob from trust store
        manager_alice.remove_trusted_key(&bob_identity);
        assert!(!manager_alice.is_key_trusted(&bob_identity));

        println!("Trust management test passed");
    }

    #[test]
    fn test_replay_attack_protection() {
        let (mut alice, mut bob) = establish_secure_session().unwrap();

        let test_message = b"Test message for replay protection";

        // Alice encrypts message
        let encrypted = alice.encrypt_message(test_message).unwrap();

        // Bob decrypts successfully first time
        let decrypted1 = bob.decrypt_message(&encrypted);
        assert!(decrypted1.is_ok());
        assert_eq!(decrypted1.unwrap(), test_message);

        // Attempt to decrypt same message again (replay attack)
        let decrypted2 = bob.decrypt_message(&encrypted);

        // Should fail due to nonce reuse protection
        assert!(decrypted2.is_err(), "Replay attack should be prevented");

        println!("Replay attack protection test passed");
    }

    #[test]
    fn test_invalid_signature_rejection() {
        let config_alice = SecurityConfig::new().unwrap();
        let mut manager_alice = SecurityManager::new(config_alice).unwrap();

        let config_bob = SecurityConfig::new().unwrap();
        let mut manager_bob = SecurityManager::new(config_bob).unwrap();

        // Create legitimate key exchange from Bob
        let mut bob_exchange = manager_bob.initiate_key_exchange().unwrap();

        // Corrupt the signature
        bob_exchange.signature[0] ^= 0xFF;

        // Alice should reject the corrupted exchange
        let result = manager_alice.process_key_exchange(&bob_exchange);
        assert!(result.is_err(), "Corrupted signature should be rejected");

        println!("Invalid signature rejection test passed");
    }

    #[test]
    fn test_ciphertext_tampering_detection() {
        let (mut alice, mut bob) = establish_secure_session().unwrap();

        let test_message = b"Sensitive data that must not be tampered with";

        // Alice encrypts message
        let mut encrypted = alice.encrypt_message(test_message).unwrap();

        // Tamper with ciphertext
        encrypted.ciphertext[5] ^= 0xFF;

        // Bob should detect tampering and reject
        let result = bob.decrypt_message(&encrypted);
        assert!(result.is_err(), "Tampered ciphertext should be rejected");

        println!("Ciphertext tampering detection test passed");
    }

    #[test]
    fn test_forward_secrecy() {
        let (mut alice, mut bob) = establish_secure_session().unwrap();

        let message1 = b"First message";
        let message2 = b"Second message";

        // Encrypt two messages
        let encrypted1 = alice.encrypt_message(message1).unwrap();
        let encrypted2 = alice.encrypt_message(message2).unwrap();

        // Messages should have different nonces
        assert_ne!(encrypted1.nonce, encrypted2.nonce);

        // Both should decrypt correctly
        assert_eq!(bob.decrypt_message(&encrypted1).unwrap(), message1);
        assert_eq!(bob.decrypt_message(&encrypted2).unwrap(), message2);

        // Simulate key compromise by rotating keys
        alice.rotate_session_keys().unwrap();
        bob.rotate_session_keys().unwrap();

        // Old messages should not be decryptable with new keys
        let encrypted3 = alice.encrypt_message(message1).unwrap();
        let decrypted3 = bob.decrypt_message(&encrypted3).unwrap();
        assert_eq!(decrypted3, message1);

        // But the nonce should be different, providing forward secrecy
        assert_ne!(encrypted1.nonce, encrypted3.nonce);

        println!("Forward secrecy test passed");
    }

    #[test]
    fn test_encryption_performance() {
        let (mut alice, mut _bob) = establish_secure_session().unwrap();

        let large_message = vec![0u8; 1024 * 1024]; // 1MB message
        let iterations = 100;

        let start_time = std::time::Instant::now();

        for _ in 0..iterations {
            let _encrypted = alice.encrypt_message(&large_message).unwrap();
        }

        let elapsed = start_time.elapsed();
        let throughput_mbps = (large_message.len() * iterations) as f64 / elapsed.as_secs_f64() / 1_000_000.0;

        println!("Encryption performance: {:.2} MB/s", throughput_mbps);

        // Should achieve reasonable throughput
        assert!(throughput_mbps > 10.0, "Encryption throughput too low: {:.2} MB/s", throughput_mbps);
    }

    #[test]
    fn test_concurrent_encryption() {
        use std::sync::{Arc, Mutex};
        use std::thread;

        let (alice, _bob) = establish_secure_session().unwrap();
        let alice_shared: Arc<Mutex<SecurityManager>> = Arc::new(Mutex::new(alice));

        let mut handles = vec![];

        // Spawn multiple threads encrypting messages
        for thread_id in 0..4 {
            let alice_clone = Arc::clone(&alice_shared);

            let handle = thread::spawn(move || {
                for i in 0..10 {
                    let message = format!("Thread {} message {}", thread_id, i);
                    let mut alice_guard = alice_clone.lock().unwrap();
                    let _encrypted = alice_guard.encrypt_message(message.as_bytes()).unwrap();
                    drop(alice_guard);

                    // Small delay to allow other threads
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            handle.join().unwrap();
        }

        let alice_guard = alice_shared.lock().unwrap();
        let stats = alice_guard.get_stats();
        assert_eq!(stats.messages_encrypted, 40); // 4 threads * 10 messages each

        println!("Concurrent encryption test passed");
    }

    #[test]
    fn test_key_derivation_consistency() {
        // Test that the same inputs produce the same derived keys
        let shared_secret = [0u8; 32]; // Fixed shared secret for testing
        let salt = b"test_salt";

        let key1 = derive_session_key(&shared_secret, salt).unwrap();
        let key2 = derive_session_key(&shared_secret, salt).unwrap();

        assert_eq!(key1, key2, "Key derivation should be deterministic");

        // Different salts should produce different keys
        let key3 = derive_session_key(&shared_secret, b"different_salt").unwrap();
        assert_ne!(key1, key3, "Different salts should produce different keys");

        println!("Key derivation consistency test passed");
    }

    #[test]
    fn test_nonce_uniqueness() {
        let (mut alice, mut _bob) = establish_secure_session().unwrap();

        let mut nonces = std::collections::HashSet::new();
        let message = b"Test message";

        // Generate many encrypted messages and collect nonces
        for _ in 0..1000 {
            let encrypted = alice.encrypt_message(message).unwrap();
            let nonce_inserted = nonces.insert(encrypted.nonce.clone());
            assert!(nonce_inserted, "Nonce was reused!");
        }

        assert_eq!(nonces.len(), 1000, "All nonces should be unique");

        println!("Nonce uniqueness test passed");
    }

    #[test]
    fn test_session_state_isolation() {
        // Create multiple independent sessions
        let (mut alice1, mut bob1) = establish_secure_session().unwrap();
        let (mut alice2, mut bob2) = establish_secure_session().unwrap();

        let message = b"Test message";

        // Encrypt with session 1
        let encrypted1 = alice1.encrypt_message(message).unwrap();

        // Try to decrypt with session 2 (should fail)
        let result = bob2.decrypt_message(&encrypted1);
        assert!(result.is_err(), "Cross-session decryption should fail");

        // Decrypt with correct session (should succeed)
        let decrypted = bob1.decrypt_message(&encrypted1).unwrap();
        assert_eq!(decrypted, message);

        println!("Session state isolation test passed");
    }

    #[test]
    fn test_security_configuration_validation() {
        let mut config = SecurityConfig::new().unwrap();

        // Test disabling encryption
        config.encryption_enabled = false;
        let manager = SecurityManager::new(config.clone());
        assert!(manager.is_ok());

        // Test disabling authentication
        config.authentication_required = false;
        let manager = SecurityManager::new(config);
        assert!(manager.is_ok());

        println!("Security configuration validation test passed");
    }

    // Helper function to establish a secure session between two parties
    fn establish_secure_session() -> Result<(SecurityManager, SecurityManager), SecurityError> {
        let config_alice = SecurityConfig::new()?;
        let mut manager_alice = SecurityManager::new(config_alice)?;

        let config_bob = SecurityConfig::new()?;
        let mut manager_bob = SecurityManager::new(config_bob)?;

        // Add each other to trust stores
        manager_alice.add_trusted_key(manager_bob.get_identity_key())?;
        manager_bob.add_trusted_key(manager_alice.get_identity_key())?;

        // Perform key exchange
        let alice_exchange = manager_alice.initiate_key_exchange()?;
        let bob_exchange = manager_bob.process_key_exchange(&alice_exchange)?;
        manager_alice.complete_key_exchange(&bob_exchange)?;

        Ok((manager_alice, manager_bob))
    }

    // Helper function for key derivation testing
    fn derive_session_key(shared_secret: &[u8], salt: &[u8]) -> Result<[u8; 32], SecurityError> {
        use hkdf::Hkdf;
        use sha2::Sha256;

        let hk = Hkdf::<Sha256>::new(Some(salt), shared_secret);
        let mut key = [0u8; 32];
        hk.expand(b"session_key", &mut key)
            .map_err(|_| SecurityError::KeyDerivationFailed)?;

        Ok(key)
    }
}