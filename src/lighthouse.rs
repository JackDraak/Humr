use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::net::{IpAddr, Ipv4Addr};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use uuid::Uuid;

// Core Lighthouse Architecture as specified in TECHNICAL_UX_REQUIREMENTS.md

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomName {
    pub adjective: String,
    pub noun: String,
    pub number: u8,
}

impl RoomName {
    pub fn generate() -> Self {
        let adjectives = [
            "sunset", "ocean", "forest", "mountain", "river", "cloud", "star", "moon",
            "dawn", "twilight", "aurora", "crystal", "silver", "golden", "emerald", "azure",
            "crimson", "amber", "jade", "pearl", "coral", "midnight", "thunder", "lightning",
            "whisper", "echo", "shadow", "bright", "calm", "swift", "gentle", "fierce",
        ];

        let nouns = [
            "dragon", "phoenix", "tiger", "eagle", "wolf", "dolphin", "hawk", "falcon",
            "bear", "lion", "panther", "raven", "swan", "deer", "fox", "owl",
            "shark", "whale", "leopard", "cheetah", "lynx", "jaguar", "cobra", "viper",
            "falcon", "condor", "osprey", "kestrel", "sparrow", "robin", "cardinal", "wren",
        ];

        let adj_idx = fastrand::usize(..adjectives.len());
        let noun_idx = fastrand::usize(..nouns.len());
        let number = fastrand::u8(0..100);

        Self {
            adjective: adjectives[adj_idx].to_string(),
            noun: nouns[noun_idx].to_string(),
            number,
        }
    }

    pub fn to_qr_data(&self) -> String {
        format!("humr://{}-{}-{:02}", self.adjective, self.noun, self.number)
    }

    pub fn pronounceable(&self) -> String {
        format!("{} {} {}", self.adjective, self.noun, self.number)
    }

    pub fn to_string(&self) -> String {
        format!("{}-{}-{:02}", self.adjective, self.noun, self.number)
    }
}

#[derive(Debug, Clone)]
pub enum DiscoveryMethod {
    BluetoothLE {
        advertising_interval: Duration,
        power_level: TxPowerLevel,
        service_uuid: Uuid,
    },
    MDNS {
        service_type: String,
        broadcast_interval: Duration,
        network_interface: NetworkInterface,
    },
    UPnP {
        port_mapping: PortMapping,
        external_port: u16,
        lease_duration: Duration,
    },
    Manual {
        host: IpAddr,
        port: u16,
        connection_method: ManualMethod,
    },
}

#[derive(Debug, Clone)]
pub enum TxPowerLevel {
    Low,    // -12 dBm, ~10m range
    Medium, // 0 dBm, ~30m range
    High,   // +4 dBm, ~100m range
}

#[derive(Debug, Clone)]
pub enum NetworkInterface {
    Default,
    WiFi,
    Ethernet,
    All,
}

#[derive(Debug, Clone)]
pub struct PortMapping {
    pub external_port: u16,
    pub internal_port: u16,
    pub protocol: PortMappingProtocol,
    pub lease_duration: Duration,
    pub description: String,
}

#[derive(Debug, Clone)]
pub enum PortMappingProtocol {
    UDP,
    TCP,
}

#[derive(Debug, Clone)]
pub enum ManualMethod {
    DirectIP,
    QRCode,
    MagicLink,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LighthouseState {
    Initializing,
    Broadcasting,
    Connected { peer_count: u8 },
    Error { error: String },
}

#[derive(Debug, Clone)]
pub struct SecurityBeacon {
    pub encryption_type: String,
    pub key_exchange: String,
    pub version: String,
}

impl SecurityBeacon {
    pub fn new() -> Self {
        Self {
            encryption_type: "chacha20poly1305".to_string(),
            key_exchange: "x25519".to_string(),
            version: "1.0".to_string(),
        }
    }
}

pub struct LighthouseService {
    pub room_name: RoomName,
    pub discovery_methods: Vec<DiscoveryMethod>,
    pub connection_state: LighthouseState,
    pub security_beacon: SecurityBeacon,
    pub qr_generator: super::discovery::QRCodeGenerator,
    pub magic_link_service: super::discovery::MagicLinkService,
    discovery_engine: DiscoveryEngine,
    event_sender: mpsc::UnboundedSender<LighthouseEvent>,
    #[allow(dead_code)]
    event_receiver: mpsc::UnboundedReceiver<LighthouseEvent>,
}

#[derive(Debug, Clone)]
pub struct LighthouseEvent {
    pub event_type: LighthouseEventType,
    pub room_name: Option<RoomName>,
    pub peer_info: Option<DiscoveredPeer>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum LighthouseEventType {
    RoomCreated,
    PeerDiscovered,
    PeerConnected,
    PeerDisconnected,
    DiscoveryMethodStarted,
    DiscoveryMethodStopped,
    Error,
}

impl LighthouseService {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        let room_name = RoomName::generate();

        Self {
            room_name,
            discovery_methods: Vec::new(),
            connection_state: LighthouseState::Initializing,
            security_beacon: SecurityBeacon::new(),
            qr_generator: super::discovery::QRCodeGenerator,
            magic_link_service: super::discovery::MagicLinkService,
            discovery_engine: DiscoveryEngine::new(tx.clone()),
            event_sender: tx,
            event_receiver: rx,
        }
    }

    pub async fn start_lighthouse(&mut self, port: u16) -> Result<()> {
        self.connection_state = LighthouseState::Broadcasting;

        // Initialize discovery methods based on specifications
        self.discovery_methods = vec![
            DiscoveryMethod::MDNS {
                service_type: "_humr._udp.local".to_string(),
                broadcast_interval: Duration::from_millis(250),
                network_interface: NetworkInterface::Default,
            },
            DiscoveryMethod::UPnP {
                port_mapping: PortMapping {
                    external_port: port,
                    internal_port: port,
                    protocol: PortMappingProtocol::UDP,
                    lease_duration: Duration::from_secs(3600),
                    description: "Humr P2P Voice Communication".to_string(),
                },
                external_port: port,
                lease_duration: Duration::from_secs(3600),
            },
            DiscoveryMethod::BluetoothLE {
                advertising_interval: Duration::from_millis(100),
                power_level: TxPowerLevel::Medium,
                service_uuid: Uuid::new_v4(), // TODO: Use fixed Humr service UUID
            },
        ];

        // Start discovery engine
        self.discovery_engine.start_broadcasting(&self.room_name, &self.discovery_methods).await?;

        // Emit room created event
        let _ = self.event_sender.send(LighthouseEvent {
            event_type: LighthouseEventType::RoomCreated,
            room_name: Some(self.room_name.clone()),
            peer_info: None,
            error: None,
        });

        Ok(())
    }

    pub async fn discover_peers(&mut self) -> Result<Vec<DiscoveredPeer>> {
        self.discovery_engine.discover_peers().await
    }

    pub fn get_room_name(&self) -> &RoomName {
        &self.room_name
    }

    pub fn get_qr_code(&self) -> Result<String> {
        super::discovery::QRCodeGenerator::generate_qr_code(&self.room_name.to_qr_data())
    }

    pub fn get_magic_link(&self) -> String {
        format!("https://humr.chat/{}", self.room_name.to_string())
    }

    pub fn get_connection_methods(&self) -> Vec<ConnectionMethod> {
        let mut methods = Vec::new();

        for discovery_method in &self.discovery_methods {
            match discovery_method {
                DiscoveryMethod::MDNS { .. } => {
                    methods.push(ConnectionMethod::LocalNetwork {
                        ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), // TODO: Get real local IP
                    });
                }
                DiscoveryMethod::UPnP { external_port, .. } => {
                    methods.push(ConnectionMethod::Internet {
                        public_ip: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 42)), // TODO: Get real external IP
                        port: *external_port,
                    });
                }
                DiscoveryMethod::BluetoothLE { .. } => {
                    methods.push(ConnectionMethod::Bluetooth {
                        device_id: "humr-device".to_string(), // TODO: Get real device ID
                    });
                }
                _ => {}
            }
        }

        methods.push(ConnectionMethod::QRCode {
            data: self.room_name.to_qr_data(),
        });

        methods.push(ConnectionMethod::MagicLink {
            url: self.get_magic_link(),
        });

        methods
    }

    pub async fn stop_lighthouse(&mut self) -> Result<()> {
        self.connection_state = LighthouseState::Initializing;
        self.discovery_engine.stop_broadcasting().await?;
        Ok(())
    }
}

// Re-use ConnectionMethod from discovery module
use super::discovery::ConnectionMethod;

#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    pub room_name: RoomName,
    pub connection_method: DiscoveryMethod,
    pub signal_strength: f32, // 0.0 - 1.0
    pub estimated_latency: Duration,
    pub security_info: SecurityBeacon,
    pub discovered_at: Instant,
}

#[derive(Debug)]
pub struct DiscoveryEngine {
    discovery_methods: VecDeque<DiscoveryMethod>,
    timeout_strategy: TimeoutStrategy,
    success_metrics: DiscoveryMetrics,
    fallback_chain: Vec<FallbackAction>,
    event_sender: mpsc::UnboundedSender<LighthouseEvent>,
    is_broadcasting: bool,
}

#[derive(Debug)]
pub struct TimeoutStrategy {
    pub bluetooth_timeout: Duration,
    pub mdns_timeout: Duration,
    pub upnp_timeout: Duration,
    pub total_timeout: Duration,
}

impl Default for TimeoutStrategy {
    fn default() -> Self {
        Self {
            bluetooth_timeout: Duration::from_secs(2),
            mdns_timeout: Duration::from_secs(3),
            upnp_timeout: Duration::from_secs(3),
            total_timeout: Duration::from_secs(8),
        }
    }
}

#[derive(Debug, Default)]
pub struct DiscoveryMetrics {
    pub bluetooth_success_rate: f32,
    pub mdns_success_rate: f32,
    pub upnp_success_rate: f32,
    pub average_discovery_time: Duration,
    pub total_discoveries: u32,
}

#[derive(Debug)]
pub enum FallbackAction {
    RetryWithLongerTimeout,
    TryAlternativeMethod,
    ShowManualInstructions,
    EnableDebugMode,
}

impl DiscoveryEngine {
    pub fn new(event_sender: mpsc::UnboundedSender<LighthouseEvent>) -> Self {
        Self {
            discovery_methods: VecDeque::new(),
            timeout_strategy: TimeoutStrategy::default(),
            success_metrics: DiscoveryMetrics::default(),
            fallback_chain: vec![
                FallbackAction::RetryWithLongerTimeout,
                FallbackAction::TryAlternativeMethod,
                FallbackAction::ShowManualInstructions,
            ],
            event_sender,
            is_broadcasting: false,
        }
    }

    pub async fn start_broadcasting(
        &mut self,
        room_name: &RoomName,
        methods: &[DiscoveryMethod],
    ) -> Result<()> {
        self.discovery_methods = methods.iter().cloned().collect();
        self.is_broadcasting = true;

        // Start broadcasting on all methods
        for method in methods {
            match method {
                DiscoveryMethod::MDNS { .. } => {
                    log::info!("Starting mDNS broadcasting for room: {}", room_name.to_string());
                    // TODO: Start actual mDNS broadcasting
                }
                DiscoveryMethod::UPnP { .. } => {
                    log::info!("Starting UPnP port mapping for room: {}", room_name.to_string());
                    // TODO: Start actual UPnP port mapping
                }
                DiscoveryMethod::BluetoothLE { .. } => {
                    log::info!("Starting Bluetooth LE advertising for room: {}", room_name.to_string());
                    // TODO: Start actual Bluetooth LE advertising
                }
                _ => {}
            }

            let _ = self.event_sender.send(LighthouseEvent {
                event_type: LighthouseEventType::DiscoveryMethodStarted,
                room_name: Some(room_name.clone()),
                peer_info: None,
                error: None,
            });
        }

        Ok(())
    }

    pub async fn discover_peers(&mut self) -> Result<Vec<DiscoveredPeer>> {
        let mut discovered_peers = Vec::new();
        let discovery_start = Instant::now();

        // Phase 1: Bluetooth LE (0-2 seconds) as specified
        if let Ok(ble_peers) = self.discover_bluetooth_le(self.timeout_strategy.bluetooth_timeout).await {
            discovered_peers.extend(ble_peers);
            if !discovered_peers.is_empty() {
                self.update_success_metrics(discovery_start.elapsed());
                return Ok(discovered_peers);
            }
        }

        // Phase 2: mDNS same network (2-5 seconds total)
        if let Ok(mdns_peers) = self.discover_mdns(self.timeout_strategy.mdns_timeout).await {
            discovered_peers.extend(mdns_peers);
            if !discovered_peers.is_empty() {
                self.update_success_metrics(discovery_start.elapsed());
                return Ok(discovered_peers);
            }
        }

        // Phase 3: Internet via UPnP (5-8 seconds total)
        if let Ok(internet_peers) = self.discover_internet(self.timeout_strategy.upnp_timeout).await {
            discovered_peers.extend(internet_peers);
        }

        self.update_success_metrics(discovery_start.elapsed());
        Ok(discovered_peers)
    }

    pub async fn discover_bluetooth_le(&self, timeout: Duration) -> Result<Vec<DiscoveredPeer>> {
        log::info!("Discovering Bluetooth LE peers (timeout: {:?})", timeout);

        // TODO: Implement actual Bluetooth LE discovery
        // For now, return mock data to match the progressive discovery pattern
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(vec![])
    }

    pub async fn discover_mdns(&self, timeout: Duration) -> Result<Vec<DiscoveredPeer>> {
        log::info!("Discovering mDNS peers (timeout: {:?})", timeout);

        // TODO: Implement actual mDNS discovery
        // This should scan for "_humr._udp.local" services
        tokio::time::sleep(Duration::from_millis(1000)).await;

        // Return a mock discovered peer to demonstrate the progressive discovery
        let mock_peer = DiscoveredPeer {
            room_name: RoomName {
                adjective: "alice".to_string(),
                noun: "livingroom".to_string(),
                number: 42,
            },
            connection_method: DiscoveryMethod::MDNS {
                service_type: "_humr._udp.local".to_string(),
                broadcast_interval: Duration::from_millis(250),
                network_interface: NetworkInterface::Default,
            },
            signal_strength: 0.95,
            estimated_latency: Duration::from_millis(2),
            security_info: SecurityBeacon::new(),
            discovered_at: Instant::now(),
        };

        Ok(vec![mock_peer])
    }

    pub async fn discover_internet(&self, timeout: Duration) -> Result<Vec<DiscoveredPeer>> {
        log::info!("Discovering internet peers via UPnP (timeout: {:?})", timeout);

        // TODO: Implement actual internet peer discovery
        tokio::time::sleep(Duration::from_millis(1500)).await;

        Ok(vec![])
    }

    pub async fn stop_broadcasting(&mut self) -> Result<()> {
        self.is_broadcasting = false;

        // Stop all broadcasting methods
        for method in &self.discovery_methods {
            match method {
                DiscoveryMethod::MDNS { .. } => {
                    log::info!("Stopping mDNS broadcasting");
                    // TODO: Stop actual mDNS broadcasting
                }
                DiscoveryMethod::UPnP { .. } => {
                    log::info!("Stopping UPnP port mapping");
                    // TODO: Remove UPnP port mapping
                }
                DiscoveryMethod::BluetoothLE { .. } => {
                    log::info!("Stopping Bluetooth LE advertising");
                    // TODO: Stop actual Bluetooth LE advertising
                }
                _ => {}
            }

            let _ = self.event_sender.send(LighthouseEvent {
                event_type: LighthouseEventType::DiscoveryMethodStopped,
                room_name: None,
                peer_info: None,
                error: None,
            });
        }

        Ok(())
    }

    fn update_success_metrics(&mut self, discovery_time: Duration) {
        self.success_metrics.total_discoveries += 1;

        // Update average discovery time
        let total_time = self.success_metrics.average_discovery_time.as_millis() as f32
            * (self.success_metrics.total_discoveries - 1) as f32;
        let new_average = (total_time + discovery_time.as_millis() as f32)
            / self.success_metrics.total_discoveries as f32;

        self.success_metrics.average_discovery_time = Duration::from_millis(new_average as u64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_name_generation() {
        let room_name = RoomName::generate();
        assert!(!room_name.adjective.is_empty());
        assert!(!room_name.noun.is_empty());
        assert!(room_name.number < 100);
    }

    #[test]
    fn test_room_name_formatting() {
        let room_name = RoomName {
            adjective: "sunset".to_string(),
            noun: "dragon".to_string(),
            number: 42,
        };

        assert_eq!(room_name.to_string(), "sunset-dragon-42");
        assert_eq!(room_name.to_qr_data(), "humr://sunset-dragon-42");
        assert_eq!(room_name.pronounceable(), "sunset dragon 42");
    }

    #[tokio::test]
    async fn test_lighthouse_service_creation() {
        let lighthouse = LighthouseService::new();
        assert_eq!(lighthouse.connection_state, LighthouseState::Initializing);
        assert!(!lighthouse.room_name.adjective.is_empty());
    }

    #[tokio::test]
    async fn test_discovery_engine_progressive_discovery() {
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut engine = DiscoveryEngine::new(tx);

        // Test progressive discovery timing
        let start = Instant::now();
        let peers = engine.discover_peers().await.unwrap();
        let elapsed = start.elapsed();

        // Should complete within 8 seconds (total timeout)
        assert!(elapsed < Duration::from_secs(9));

        // Should find at least the mock mDNS peer
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].room_name.adjective, "alice");
    }
}