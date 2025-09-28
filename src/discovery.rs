use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoomInfo {
    pub room_id: String,
    pub display_name: String,
    pub host_name: String,
    pub port: u16,
    pub is_encrypted: bool,
    pub max_participants: u8,
    pub current_participants: u8,
    #[serde(skip, default = "std::time::Instant::now")]
    pub created_at: Instant,
    pub connection_methods: Vec<ConnectionMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionMethod {
    LocalNetwork { ip: IpAddr },
    Internet { public_ip: IpAddr, port: u16 },
    Bluetooth { device_id: String },
    QRCode { data: String },
    MagicLink { url: String },
}

#[derive(Debug, Clone)]
pub struct DiscoveryEvent {
    pub event_type: DiscoveryEventType,
    pub room_info: Option<RoomInfo>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub enum DiscoveryEventType {
    RoomDiscovered,
    RoomLost,
    RoomUpdated,
    Error,
}

pub struct DiscoveryManager {
    room_registry: Arc<RwLock<HashMap<String, RoomInfo>>>,
    event_sender: mpsc::UnboundedSender<DiscoveryEvent>,
    #[allow(dead_code)]
    event_receiver: mpsc::UnboundedReceiver<DiscoveryEvent>,
    mdns_service: Option<MDNSService>,
    upnp_service: Option<UPnPService>,
    is_running: bool,
}

impl DiscoveryManager {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        Self {
            room_registry: Arc::new(RwLock::new(HashMap::new())),
            event_sender: tx,
            event_receiver: rx,
            mdns_service: None,
            upnp_service: None,
            is_running: false,
        }
    }

    pub async fn start(&mut self) -> Result<()> {
        if self.is_running {
            return Ok(());
        }

        // Initialize mDNS service
        self.mdns_service = Some(MDNSService::new(self.event_sender.clone()).await?);

        // Initialize UPnP service
        self.upnp_service = Some(UPnPService::new().await?);

        self.is_running = true;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        if !self.is_running {
            return Ok(());
        }

        if let Some(mdns) = &mut self.mdns_service {
            mdns.stop().await?;
        }

        self.is_running = false;
        Ok(())
    }

    pub async fn create_room(&mut self, display_name: String, port: u16) -> Result<RoomInfo> {
        let room_id = generate_room_code();
        let host_name = get_hostname().unwrap_or_else(|| "Unknown".to_string());

        let mut connection_methods = Vec::new();

        // Add local network method
        if let Ok(local_ip) = get_local_ip() {
            connection_methods.push(ConnectionMethod::LocalNetwork { ip: local_ip });
        }

        // Try UPnP for internet access
        if let Some(upnp) = &self.upnp_service {
            if let Ok(public_ip) = upnp.get_external_ip().await {
                if upnp.forward_port(port).await.is_ok() {
                    connection_methods.push(ConnectionMethod::Internet { public_ip, port });
                }
            }
        }

        // Generate QR code data
        let qr_data = format!("humr://{}/{}/{}", room_id, host_name, port);
        connection_methods.push(ConnectionMethod::QRCode { data: qr_data.clone() });

        // Generate magic link
        let magic_link = format!("https://humr.chat/{}", room_id);
        connection_methods.push(ConnectionMethod::MagicLink { url: magic_link });

        let room_info = RoomInfo {
            room_id: room_id.clone(),
            display_name,
            host_name,
            port,
            is_encrypted: true,
            max_participants: 10,
            current_participants: 1,
            created_at: Instant::now(),
            connection_methods,
        };

        // Register room
        {
            let mut registry = self.room_registry.write().await;
            registry.insert(room_id.clone(), room_info.clone());
        }

        // Start broadcasting via mDNS
        if let Some(mdns) = &mut self.mdns_service {
            mdns.advertise_room(&room_info).await?;
        }

        Ok(room_info)
    }

    pub async fn discover_rooms(&self) -> Result<Vec<RoomInfo>> {
        let registry = self.room_registry.read().await;
        Ok(registry.values().cloned().collect())
    }

    pub async fn join_room(&self, room_id: &str) -> Result<RoomInfo> {
        let registry = self.room_registry.read().await;
        registry
            .get(room_id)
            .cloned()
            .ok_or_else(|| anyhow!("Room not found: {}", room_id))
    }

    pub fn get_event_sender(&self) -> mpsc::UnboundedSender<DiscoveryEvent> {
        self.event_sender.clone()
    }
}

pub struct MDNSService {
    event_sender: mpsc::UnboundedSender<DiscoveryEvent>,
    #[allow(dead_code)]
    advertised_rooms: HashMap<String, RoomInfo>,
}

impl MDNSService {
    pub async fn new(event_sender: mpsc::UnboundedSender<DiscoveryEvent>) -> Result<Self> {
        Ok(Self {
            event_sender,
            advertised_rooms: HashMap::new(),
        })
    }

    pub async fn advertise_room(&mut self, room_info: &RoomInfo) -> Result<()> {
        // In a real implementation, this would use the mdns crate to broadcast
        // the room information on the local network

        log::info!("Advertising room {} via mDNS", room_info.room_id);

        // Simulate mDNS advertisement
        self.advertised_rooms.insert(room_info.room_id.clone(), room_info.clone());

        // Notify about our own room
        let _ = self.event_sender.send(DiscoveryEvent {
            event_type: DiscoveryEventType::RoomDiscovered,
            room_info: Some(room_info.clone()),
            error: None,
        });

        Ok(())
    }

    pub async fn discover_rooms(&self) -> Result<Vec<RoomInfo>> {
        // In a real implementation, this would scan for mDNS services
        // of type "_humr._tcp.local"

        log::info!("Discovering rooms via mDNS");

        // Simulate discovery of some rooms
        let mut rooms = Vec::new();

        // Add some mock discovered rooms for demo
        let mock_room = RoomInfo {
            room_id: "alice-living-room".to_string(),
            display_name: "Alice's Living Room Chat".to_string(),
            host_name: "Alice's MacBook".to_string(),
            port: 8080,
            is_encrypted: true,
            max_participants: 5,
            current_participants: 1,
            created_at: Instant::now(),
            connection_methods: vec![
                ConnectionMethod::LocalNetwork {
                    ip: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))
                },
            ],
        };
        rooms.push(mock_room);

        Ok(rooms)
    }

    pub async fn stop(&mut self) -> Result<()> {
        log::info!("Stopping mDNS service");
        self.advertised_rooms.clear();
        Ok(())
    }
}

pub struct UPnPService {
    #[allow(dead_code)]
    gateway: Option<String>,
}

impl UPnPService {
    pub async fn new() -> Result<Self> {
        // In a real implementation, this would discover UPnP gateways
        log::info!("Initializing UPnP service");

        Ok(Self {
            gateway: Some("192.168.1.1".to_string()),
        })
    }

    pub async fn forward_port(&self, port: u16) -> Result<()> {
        // In a real implementation, this would use the igd crate to
        // automatically forward the specified port through UPnP

        log::info!("Attempting UPnP port forwarding for port {}", port);

        // Simulate successful port forwarding
        tokio::time::sleep(Duration::from_millis(500)).await;

        log::info!("Successfully forwarded port {} via UPnP", port);
        Ok(())
    }

    pub async fn get_external_ip(&self) -> Result<IpAddr> {
        // In a real implementation, this would query the router for the external IP

        log::info!("Querying external IP via UPnP");

        // Return a mock external IP
        Ok(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 42)))
    }

    pub async fn remove_port_forwarding(&self, port: u16) -> Result<()> {
        log::info!("Removing UPnP port forwarding for port {}", port);

        // Simulate cleanup
        tokio::time::sleep(Duration::from_millis(200)).await;

        Ok(())
    }
}

pub struct QRCodeGenerator;

impl QRCodeGenerator {
    pub fn generate_qr_code(data: &str) -> Result<String> {
        use qrcode::QrCode;

        let code = QrCode::new(data.as_bytes())?;
        let image = code.render::<char>()
            .quiet_zone(false)
            .module_dimensions(1, 1)
            .build();

        Ok(image)
    }

    pub fn generate_connection_qr(room_info: &RoomInfo) -> Result<String> {
        let connection_data = serde_json::to_string(room_info)?;
        Self::generate_qr_code(&connection_data)
    }
}

pub struct MagicLinkService;

impl MagicLinkService {
    pub fn generate_magic_link(room_info: &RoomInfo) -> String {
        format!("humr://{}", room_info.room_id)
    }

    pub fn parse_magic_link(link: &str) -> Result<String> {
        if let Some(room_id) = link.strip_prefix("humr://") {
            Ok(room_id.to_string())
        } else {
            Err(anyhow!("Invalid magic link format"))
        }
    }

    pub fn generate_universal_link(room_info: &RoomInfo) -> String {
        format!("https://humr.chat/{}?host={}&port={}",
                room_info.room_id,
                room_info.host_name,
                room_info.port)
    }
}

// Utility functions

pub fn generate_room_code() -> String {
    let adjectives = ["sunset", "ocean", "mountain", "forest", "river", "cloud", "star", "moon"];
    let animals = ["dragon", "phoenix", "tiger", "eagle", "wolf", "dolphin", "hawk", "falcon"];
    let numbers = ["07", "23", "42", "73", "91", "13", "17", "29"];

    let adj = adjectives[fastrand::usize(..adjectives.len())];
    let animal = animals[fastrand::usize(..animals.len())];
    let num = numbers[fastrand::usize(..numbers.len())];

    format!("{}-{}-{}", adj, animal, num)
}

pub fn get_local_ip() -> Result<IpAddr> {
    // In a real implementation, this would discover the actual local IP
    // For now, return a mock local IP
    Ok(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)))
}

pub fn get_hostname() -> Option<String> {
    // In a real implementation, this would get the actual hostname
    Some("Local Device".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discovery_manager_creation() {
        let manager = DiscoveryManager::new();
        assert!(!manager.is_running);
    }

    #[tokio::test]
    async fn test_room_code_generation() {
        let room_code = generate_room_code();
        assert!(room_code.contains('-'));
        assert!(room_code.len() > 10);
    }

    #[tokio::test]
    async fn test_magic_link_parsing() {
        let room_id = "sunset-dragon-42";
        let link = format!("humr://{}", room_id);

        let parsed = MagicLinkService::parse_magic_link(&link).unwrap();
        assert_eq!(parsed, room_id);
    }

    #[test]
    fn test_qr_code_generation() {
        let test_data = "humr://test-room-123";
        let qr_result = QRCodeGenerator::generate_qr_code(test_data);
        assert!(qr_result.is_ok());
    }
}