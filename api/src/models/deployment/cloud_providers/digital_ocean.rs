use std::net::{Ipv4Addr, Ipv6Addr};

use serde::{Deserialize, Serialize};

// TODO: create a enum or struct for
// region:
// Datacenter Name  Geographic Location             Slug (for the API and doctl) 
// NYC1             New York City, United States    nyc1
// NYC2             New York City, United States    nyc2
// NYC3             New York City, United States    nyc3
// AMS2             Amsterdam, the Netherlands      ams2
// AMS3             Amsterdam, the Netherlands      ams3
// SFO1             San Francisco, United States    sfo1
// SFO2             San Francisco, United States    sfo2
// SFO3             San Francisco, United States    sfo3
// SGP1             Singapore                       sgp1
// LON1             London, United Kingdom          lon1
// FRA1             Frankfurt, Germany              fra1
// TOR1             Toronto, Canada                 tor1
// BLR1             Bangalore, India                blr1

#[derive(Debug, Deserialize, Serialize)]
pub struct DigitalOceanImage {
	pub id: String,
	pub distribution: String,
	pub slug: String,
	pub public: bool,
	pub regions: Vec<String>,
	pub created_at: String,
	pub r#type: String,
	pub min_disk_size: u32,
	pub size_gigabytes: f64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub enum DropletStatus {
	New,
	Active,
	Off,
	Archive,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IpV4Address {
	pub ip_address: Ipv4Addr,
	pub netmask: Ipv4Addr,
	pub gateway: Ipv4Addr,
	pub r#type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IpV6Address {
	pub ip_address: Ipv6Addr,
	pub netmask: Ipv6Addr,
	pub gateway: Ipv6Addr,
	pub r#type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NetworkDetails {
	pub v4: Vec<IpV4Address>,
	pub v6: Vec<IpV6Address>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NextBackupWindow {
	pub start: String,
	pub end: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegionDetails {
	pub name: String,
	pub slug: String,
	pub sizes: Vec<String>,
	pub features: Vec<String>,
	pub available: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct SizeObject {
	pub slug: String,
	pub memory: String,
	pub vcpus: u16,
	pub disk: u64,
	pub transfer: f64,
	pub price_monthly: f64,
	pub price_hourly: f64,
	pub regions: Vec<String>,
	pub available: bool,
	pub description: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DropletDetails {
	pub id: String,
	pub name: String,
	pub memory: u64,
	pub vcpus: u16,
	pub disk: u16,
	pub locked: bool,
	pub created_at: String,
	pub status: String,
	pub backup_ids: Vec<String>,
	pub snapshot_ids: Vec<String>,
	pub features: Vec<String>,
	pub region: Vec<RegionDetails>,
	pub image: DigitalOceanImage,
	pub size: SizeObject,
	pub size_slug: String,
	pub networks: NetworkDetails,
	pub next_backup_window: NextBackupWindow,
	pub tags: Vec<String>,
	pub vpc_uuid: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct DropletRequest {
	pub region: String,
	pub name: String,
	pub size: String,
	pub image: String,
	// stores ssh fingerprint
	// after image, these details are not mandatory
	pub ssh_keys: Option<Vec<String>>,
	pub backups: Option<bool>,
	pub ipv6: Option<bool>,
	pub private_networking: Option<bool>,
	pub vpc_uuid: Option<String>,
	pub user_data: Option<String>,
	pub volumes: Option<Vec<String>>,
	pub tags: Option<Vec<String>>
}