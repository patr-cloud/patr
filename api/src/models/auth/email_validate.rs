use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct IpQualityScore {
	pub valid: bool,
	pub disposable: bool,
}
