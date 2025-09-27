# Humr Voice Communication - Security Architecture

## Overview

Humr implements robust end-to-end encryption and authentication to ensure secure voice communication between peers. The security model is designed to protect against eavesdropping, man-in-the-middle attacks, and ensure communication integrity.

## Cryptographic Standards

### Key Exchange
- **X25519 Elliptic Curve Diffie-Hellman** for ephemeral key exchange
- **Ed25519** for digital signatures and identity verification
- Fresh ephemeral keys generated for each session (perfect forward secrecy)

### Encryption
- **ChaCha20-Poly1305** AEAD cipher for audio stream encryption
- **SHA-256** for key derivation and message authentication
- **Cryptographically secure random number generation** using OS entropy

### Key Derivation
- HKDF-like approach combining:
  - Shared secret from X25519 DH exchange
  - Both peers' identity public keys
  - Protocol version string ("HUMR_SESSION_KEY_V1")

## Security Protocol

### 1. Identity Management
Each user has a long-term Ed25519 identity keypair:
```
Identity Private Key: 32 bytes (stored securely on device)
Identity Public Key: 32 bytes (shared with contacts)
```

### 2. Trust Establishment
- **Manual verification** of peer identity keys (recommended)
- **TOFU (Trust On First Use)** for development/testing
- Trusted peers stored in local configuration
- Future: PKI integration for organizational deployments

### 3. Session Establishment

#### Handshake Protocol:
1. **Initiator** generates ephemeral X25519 keypair
2. **Initiator** sends:
   - Identity public key
   - Ephemeral public key
   - Signature over (ephemeral_key || timestamp)
   - Timestamp (replay protection)

3. **Responder** verifies:
   - Signature validity
   - Timestamp freshness (5-minute window)
   - Peer identity is trusted

4. **Responder** responds with:
   - Own ephemeral public key
   - Signature over (ephemeral_key || timestamp)
   - Timestamp

5. **Both parties** derive session key:
   ```
   shared_secret = X25519(my_ephemeral_private, peer_ephemeral_public)
   session_key = SHA256(shared_secret || my_identity || peer_identity || "HUMR_SESSION_KEY_V1")
   ```

### 4. Audio Stream Encryption

Each audio frame is encrypted independently:
```
nonce = 12 random bytes (generated per frame)
ciphertext = ChaCha20Poly1305.encrypt(session_key, nonce, audio_data)
message = {
  "type": "EncryptedAudio",
  "nonce": base64(nonce),
  "ciphertext": base64(ciphertext),
  "frame_number": counter
}
```

## Security Properties

### Confidentiality
- ✅ **End-to-end encryption** - Only communicating parties can decrypt audio
- ✅ **Forward secrecy** - Compromise of long-term keys doesn't affect past sessions
- ✅ **Per-frame encryption** - Each audio frame encrypted with unique nonce

### Integrity
- ✅ **Authenticated encryption** - ChaCha20-Poly1305 provides integrity protection
- ✅ **Digital signatures** - All handshake messages are cryptographically signed
- ✅ **Replay protection** - Timestamps prevent replay of handshake messages

### Authentication
- ✅ **Peer identity verification** - Ed25519 signatures prove identity
- ✅ **Trust establishment** - Only trusted peers can establish sessions
- ✅ **Session binding** - Session keys derived from both identities

## Threat Model

### Protected Against:
- **Passive eavesdropping** - All communications encrypted
- **Active MITM attacks** - Identity verification prevents impersonation
- **Replay attacks** - Timestamps and nonces prevent replay
- **Key compromise** - Forward secrecy limits damage from key exposure

### Current Limitations:
- **Metadata protection** - Connection timing/patterns may be observable
- **Traffic analysis** - Packet sizes and timing could leak information
- **Denial of service** - No specific DoS protection implemented
- **Key distribution** - Manual key exchange required for trust establishment

## Implementation Notes

### Key Storage
```rust
// ASSUMPTION: Keys stored in memory only for proof-of-concept
// Production implementation should use:
// - Hardware security modules (HSM) when available
// - OS keychain integration (macOS Keychain, Windows DPAPI)
// - Encrypted storage with user-derived key
```

### Trust Model
```rust
// ASSUMPTION: Development uses automatic trust-on-first-use
// Production deployment options:
// 1. Manual key verification (Signal-style)
// 2. PKI with certificate authority
// 3. Web of trust (PGP-style)
// 4. Blockchain-based key transparency
```

### Cryptographic Libraries
- **ring** - Core cryptographic primitives
- **x25519-dalek** - X25519 key exchange
- **ed25519-dalek** - Ed25519 signatures
- **chacha20poly1305** - AEAD encryption
- All libraries are well-audited and widely used

## Security Recommendations

### For Users:
1. **Verify peer identities** manually when possible
2. **Keep software updated** to receive security patches
3. **Use trusted networks** when establishing connections
4. **Report suspicious behavior** (unexpected key changes, etc.)

### For Developers:
1. **Regular security audits** of cryptographic code
2. **Constant-time operations** to prevent timing attacks
3. **Secure key generation** using OS entropy sources
4. **Input validation** on all network messages
5. **Memory clearing** of sensitive data after use

### For Deployment:
1. **TLS transport security** for additional protection
2. **Network monitoring** for anomaly detection
3. **Key rotation policies** for long-term deployments
4. **Incident response plan** for security breaches

## Future Enhancements

### Planned Security Features:
- **Post-quantum cryptography** (Kyber/Dilithium integration)
- **Metadata protection** (traffic padding, timing obfuscation)
- **Key transparency** (public key logging/verification)
- **Multi-device support** (key synchronization)
- **Group communications** (secure multi-party protocols)

### Compliance Considerations:
- **GDPR compliance** for European deployments
- **HIPAA compliance** for healthcare usage
- **SOX compliance** for financial communications
- **Export control** regulations for cryptographic software

## Security Contact

For security vulnerabilities or concerns:
- **Author**: JackDraak@example.com
- **Security Response**: Create issue with "SECURITY" tag
- **PGP Key**: [To be established for production]

---

**⚠️ Security Notice**: This is a proof-of-concept implementation. While the cryptographic design follows industry best practices, a full security audit is required before production deployment.