use std::collections::HashMap;

use api_models::utils::Uuid;
use once_cell::sync::OnceCell;

pub mod cloud_providers;

pub const DEFAULT_MACHINE_TYPES: [(i16, i32); 5] = [
	(1, 2),  // 1 vCPU, 0.5 GB RAM
	(1, 4),  // 1 vCPU, 1 GB RAM
	(1, 8),  // 1 vCPU, 2 GB RAM
	(2, 8),  // 2 vCPU, 4 GB RAM
	(4, 32), // 4 vCPU, 8 GB RAM
];

pub static MACHINE_TYPES: OnceCell<HashMap<Uuid, (i16, i32)>> = OnceCell::new();
