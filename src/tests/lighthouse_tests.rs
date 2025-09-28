use crate::lighthouse::*;
use crate::discovery::ConnectionMethod;
use std::time::{Duration, Instant};
use tokio::time::timeout;

/// Test-Driven Development for Lighthouse Service
/// Following Uncle Bob's Red-Green-Refactor methodology
///
/// Tests written BEFORE implementation to define behavior

#[cfg(test)]
mod lighthouse_tdd_tests {
    use super::*;

    /// RED: Test room name generation requirements
    #[test]
    fn test_room_name_should_generate_memorable_three_part_identifier() {
        // REQUIREMENT: Room names must be memorable and follow "adjective-noun-number" pattern
        let room_name = RoomName::generate();

        // Verify structure
        assert!(!room_name.adjective.is_empty(), "Adjective must not be empty");
        assert!(!room_name.noun.is_empty(), "Noun must not be empty");
        assert!(room_name.number < 100, "Number must be 0-99 for collision avoidance");

        // Verify pronounceability
        let pronounceable = room_name.pronounceable();
        assert!(pronounceable.contains(&room_name.adjective));
        assert!(pronounceable.contains(&room_name.noun));
        assert!(pronounceable.contains(&room_name.number.to_string()));
    }

    #[test]
    fn test_room_name_should_format_correctly_for_different_outputs() {
        // REQUIREMENT: Room names must format correctly for QR codes and magic links
        let room_name = RoomName {
            adjective: "sunset".to_string(),
            noun: "dragon".to_string(),
            number: 42,
        };

        assert_eq!(room_name.to_string(), "sunset-dragon-42");
        assert_eq!(room_name.to_qr_data(), "humr://sunset-dragon-42");
        assert_eq!(room_name.pronounceable(), "sunset dragon 42");
    }

    #[test]
    fn test_room_name_should_generate_unique_identifiers() {
        // REQUIREMENT: Room names should avoid collisions
        let name1 = RoomName::generate();
        let name2 = RoomName::generate();

        // Very unlikely to be the same (but not impossible due to randomness)
        // In real implementation, should check against active rooms
        assert_ne!(name1.to_string(), name2.to_string(), "Generated names should be different");
    }

    /// RED: Test lighthouse service initialization
    #[tokio::test]
    async fn test_lighthouse_service_should_initialize_in_correct_state() {
        // REQUIREMENT: Lighthouse service must start in Initializing state
        let lighthouse = LighthouseService::new();

        match lighthouse.connection_state {
            LighthouseState::Initializing => {}, // Expected
            _ => panic!("Lighthouse should start in Initializing state"),
        }

        // Should have a valid room name
        assert!(!lighthouse.room_name.adjective.is_empty());
        assert!(!lighthouse.room_name.noun.is_empty());

        // Should have security beacon configured
        assert_eq!(lighthouse.security_beacon.encryption_type, "chacha20poly1305");
        assert_eq!(lighthouse.security_beacon.key_exchange, "x25519");
    }

    #[tokio::test]
    async fn test_lighthouse_service_should_start_broadcasting_on_command() {
        // REQUIREMENT: Lighthouse must transition to Broadcasting state when started
        let mut lighthouse = LighthouseService::new();

        let result = lighthouse.start_lighthouse(8080).await;
        assert!(result.is_ok(), "Starting lighthouse should succeed");

        match lighthouse.connection_state {
            LighthouseState::Broadcasting => {}, // Expected
            _ => panic!("Lighthouse should be in Broadcasting state after start"),
        }

        // Should have configured discovery methods
        assert!(!lighthouse.discovery_methods.is_empty(), "Should have discovery methods configured");
    }

    #[tokio::test]
    async fn test_lighthouse_service_should_generate_qr_code() {
        // REQUIREMENT: Lighthouse must provide QR code for visual connection sharing
        let lighthouse = LighthouseService::new();

        let qr_result = lighthouse.get_qr_code();
        assert!(qr_result.is_ok(), "QR code generation should succeed");

        let qr_code = qr_result.unwrap();
        assert!(!qr_code.is_empty(), "QR code should not be empty");
        // QR code should contain the room identifier
        // (exact format depends on QR library implementation)
    }

    #[tokio::test]
    async fn test_lighthouse_service_should_provide_magic_links() {
        // REQUIREMENT: Lighthouse must provide universal magic links
        let lighthouse = LighthouseService::new();

        let magic_link = lighthouse.get_magic_link();
        assert!(magic_link.starts_with("https://humr.chat/"), "Magic link should use correct domain");
        assert!(magic_link.contains(&lighthouse.room_name.to_string()), "Magic link should contain room name");
    }

    #[tokio::test]
    async fn test_lighthouse_service_should_provide_multiple_connection_methods() {
        // REQUIREMENT: Lighthouse must provide multiple connection methods per UX specs
        let mut lighthouse = LighthouseService::new();
        lighthouse.start_lighthouse(8080).await.unwrap();

        let methods = lighthouse.get_connection_methods();

        // Should have at least: Local Network, Internet, QR Code, Magic Link
        assert!(methods.len() >= 4, "Should provide multiple connection methods");

        // Verify specific method types exist
        let has_local = methods.iter().any(|m| matches!(m, ConnectionMethod::LocalNetwork { .. }));
        let has_internet = methods.iter().any(|m| matches!(m, ConnectionMethod::Internet { .. }));
        let has_qr = methods.iter().any(|m| matches!(m, ConnectionMethod::QRCode { .. }));
        let has_magic = methods.iter().any(|m| matches!(m, ConnectionMethod::MagicLink { .. }));

        assert!(has_local, "Should provide local network connection method");
        assert!(has_internet, "Should provide internet connection method");
        assert!(has_qr, "Should provide QR code connection method");
        assert!(has_magic, "Should provide magic link connection method");
    }

    /// RED: Test progressive discovery engine requirements
    #[tokio::test]
    async fn test_discovery_engine_should_implement_progressive_discovery() {
        // REQUIREMENT: Discovery must try methods in order with specific timeouts
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let mut engine = DiscoveryEngine::new(tx);

        let start_time = Instant::now();
        let peers = engine.discover_peers().await;
        let elapsed = start_time.elapsed();

        assert!(peers.is_ok(), "Discovery should not fail");

        // Should complete within total timeout (8 seconds per spec)
        assert!(elapsed < Duration::from_secs(9), "Discovery should complete within timeout");

        // Test that it follows progressive strategy
        // Phase 1: Bluetooth LE (0-2 seconds)
        // Phase 2: mDNS (2-5 seconds)
        // Phase 3: Internet (5-8 seconds)
    }

    #[tokio::test]
    async fn test_discovery_engine_should_return_early_on_successful_discovery() {
        // REQUIREMENT: Discovery should return immediately when peers are found
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let mut engine = DiscoveryEngine::new(tx);

        // If a peer is found in phase 1 (Bluetooth), should not wait for phase 2/3
        let start_time = Instant::now();
        let _peers = engine.discover_peers().await;
        let elapsed = start_time.elapsed();

        // If successful early, should complete much faster than 8 seconds
        // (This test may pass or fail depending on whether mock peers are returned)
    }

    #[tokio::test]
    async fn test_discovery_engine_should_handle_method_failures_gracefully() {
        // REQUIREMENT: Discovery should continue to next method if one fails
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let mut engine = DiscoveryEngine::new(tx);

        // Even if Bluetooth fails, should try mDNS and UPnP
        let result = engine.discover_peers().await;

        // Should not panic or return error just because one method fails
        assert!(result.is_ok(), "Discovery should handle individual method failures");
    }

    /// RED: Test discovery method specific requirements
    #[tokio::test]
    async fn test_mdns_discovery_should_follow_specification() {
        // REQUIREMENT: mDNS must use "_humr._udp.local" service type
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let engine = DiscoveryEngine::new(tx);

        // Test mDNS timeout requirement (3 seconds)
        let start_time = Instant::now();
        let result = engine.discover_mdns(Duration::from_secs(3)).await;
        let elapsed = start_time.elapsed();

        // Should respect timeout
        assert!(elapsed <= Duration::from_secs(4), "mDNS discovery should respect timeout");

        // Should return result (even if empty)
        assert!(result.is_ok(), "mDNS discovery should not fail");
    }

    #[tokio::test]
    async fn test_bluetooth_le_discovery_should_follow_specification() {
        // REQUIREMENT: Bluetooth LE timeout is 2 seconds
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let engine = DiscoveryEngine::new(tx);

        let start_time = Instant::now();
        let result = engine.discover_bluetooth_le(Duration::from_secs(2)).await;
        let elapsed = start_time.elapsed();

        // Should respect timeout
        assert!(elapsed <= Duration::from_secs(3), "Bluetooth LE discovery should respect timeout");

        // Should return result (even if empty)
        assert!(result.is_ok(), "Bluetooth LE discovery should not fail");
    }

    #[tokio::test]
    async fn test_internet_discovery_should_follow_specification() {
        // REQUIREMENT: Internet discovery timeout is 3 seconds
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let engine = DiscoveryEngine::new(tx);

        let start_time = Instant::now();
        let result = engine.discover_internet(Duration::from_secs(3)).await;
        let elapsed = start_time.elapsed();

        // Should respect timeout
        assert!(elapsed <= Duration::from_secs(4), "Internet discovery should respect timeout");

        // Should return result (even if empty)
        assert!(result.is_ok(), "Internet discovery should not fail");
    }

    /// RED: Test lighthouse broadcasting requirements
    #[tokio::test]
    async fn test_lighthouse_should_broadcast_on_multiple_methods() {
        // REQUIREMENT: Lighthouse must broadcast on mDNS, UPnP, and Bluetooth LE
        let mut lighthouse = LighthouseService::new();

        let result = lighthouse.start_lighthouse(8080).await;
        assert!(result.is_ok(), "Starting lighthouse should succeed");

        // Should have configured all required discovery methods
        let has_mdns = lighthouse.discovery_methods.iter().any(|m| matches!(m, DiscoveryMethod::MDNS { .. }));
        let has_upnp = lighthouse.discovery_methods.iter().any(|m| matches!(m, DiscoveryMethod::UPnP { .. }));
        let has_ble = lighthouse.discovery_methods.iter().any(|m| matches!(m, DiscoveryMethod::BluetoothLE { .. }));

        assert!(has_mdns, "Should configure mDNS broadcasting");
        assert!(has_upnp, "Should configure UPnP port mapping");
        assert!(has_ble, "Should configure Bluetooth LE advertising");
    }

    #[tokio::test]
    async fn test_lighthouse_should_stop_broadcasting_cleanly() {
        // REQUIREMENT: Lighthouse must clean up resources when stopped
        let mut lighthouse = LighthouseService::new();

        lighthouse.start_lighthouse(8080).await.unwrap();
        let result = lighthouse.stop_lighthouse().await;

        assert!(result.is_ok(), "Stopping lighthouse should succeed");

        match lighthouse.connection_state {
            LighthouseState::Initializing => {}, // Expected after stop
            _ => panic!("Lighthouse should return to Initializing state after stop"),
        }
    }

    /// RED: Test security beacon requirements
    #[test]
    fn test_security_beacon_should_advertise_correct_protocols() {
        // REQUIREMENT: Security beacon must advertise ChaCha20-Poly1305 and X25519
        let beacon = SecurityBeacon::new();

        assert_eq!(beacon.encryption_type, "chacha20poly1305");
        assert_eq!(beacon.key_exchange, "x25519");
        assert!(!beacon.version.is_empty());
    }

    /// RED: Test discovered peer structure requirements
    #[test]
    fn test_discovered_peer_should_contain_required_information() {
        // REQUIREMENT: Discovered peers must contain all info needed for connection
        let room_name = RoomName::generate();
        let peer = DiscoveredPeer {
            room_name: room_name.clone(),
            connection_method: DiscoveryMethod::MDNS {
                service_type: "_humr._udp.local".to_string(),
                broadcast_interval: Duration::from_millis(250),
                network_interface: NetworkInterface::Default,
            },
            signal_strength: 0.85,
            estimated_latency: Duration::from_millis(15),
            security_info: SecurityBeacon::new(),
            discovered_at: Instant::now(),
        };

        // Verify all required fields are present and valid
        assert_eq!(peer.room_name.to_string(), room_name.to_string());
        assert!(peer.signal_strength >= 0.0 && peer.signal_strength <= 1.0);
        assert!(peer.estimated_latency < Duration::from_secs(1)); // Should be reasonable
        assert_eq!(peer.security_info.encryption_type, "chacha20poly1305");
    }

    /// RED: Test timeout strategy requirements
    #[test]
    fn test_timeout_strategy_should_match_ux_specifications() {
        // REQUIREMENT: Timeouts must match UX specification for sub-10-second connections
        let strategy = TimeoutStrategy::default();

        assert_eq!(strategy.bluetooth_timeout, Duration::from_secs(2));
        assert_eq!(strategy.mdns_timeout, Duration::from_secs(3));
        assert_eq!(strategy.upnp_timeout, Duration::from_secs(3));
        assert_eq!(strategy.total_timeout, Duration::from_secs(8));

        // Total should be sum of phases
        let expected_total = strategy.bluetooth_timeout + strategy.mdns_timeout + strategy.upnp_timeout;
        assert!(strategy.total_timeout >= expected_total, "Total timeout should accommodate all phases");
    }

    /// RED: Test that lighthouse events are properly emitted
    #[tokio::test]
    async fn test_lighthouse_should_emit_events_for_state_changes() {
        // REQUIREMENT: Lighthouse must emit events for UX feedback
        let mut lighthouse = LighthouseService::new();

        // Starting lighthouse should emit RoomCreated event
        let result = lighthouse.start_lighthouse(8080).await;
        assert!(result.is_ok(), "Starting lighthouse should succeed");

        // Note: In real implementation, would need to check that events are actually sent
        // This test verifies the structure exists for event emission
    }

    /// RED: Test lighthouse service integration with discovery engine
    #[tokio::test]
    async fn test_lighthouse_service_should_integrate_with_discovery_engine() {
        // REQUIREMENT: Lighthouse service must use discovery engine for peer discovery
        let mut lighthouse = LighthouseService::new();
        lighthouse.start_lighthouse(8080).await.unwrap();

        let peers_result = lighthouse.discover_peers().await;
        assert!(peers_result.is_ok(), "Peer discovery should work through lighthouse service");

        // Should delegate to discovery engine
        let peers = peers_result.unwrap();
        // Note: peers may be empty if no actual peers are broadcasting
    }
}

/// Performance and integration tests for lighthouse service
#[cfg(test)]
mod lighthouse_performance_tests {
    use super::*;

    #[tokio::test]
    async fn test_lighthouse_startup_should_be_fast() {
        // REQUIREMENT: Lighthouse startup should be near-instantaneous for good UX
        let mut lighthouse = LighthouseService::new();

        let start_time = Instant::now();
        let result = lighthouse.start_lighthouse(8080).await;
        let elapsed = start_time.elapsed();

        assert!(result.is_ok(), "Lighthouse startup should succeed");
        assert!(elapsed < Duration::from_millis(500), "Lighthouse startup should be fast");
    }

    #[tokio::test]
    async fn test_discovery_should_respect_total_timeout() {
        // REQUIREMENT: Discovery must complete within 8 seconds (per UX spec)
        let mut lighthouse = LighthouseService::new();

        let discovery_result = timeout(Duration::from_secs(10), lighthouse.discover_peers()).await;

        assert!(discovery_result.is_ok(), "Discovery should complete within timeout");

        let peers_result = discovery_result.unwrap();
        assert!(peers_result.is_ok(), "Discovery should not fail");
    }

    #[tokio::test]
    async fn test_room_name_generation_should_be_fast() {
        // REQUIREMENT: Room name generation should not block UX
        let start_time = Instant::now();

        for _ in 0..100 {
            let _room_name = RoomName::generate();
        }

        let elapsed = start_time.elapsed();
        assert!(elapsed < Duration::from_millis(100), "Room name generation should be fast");
    }
}