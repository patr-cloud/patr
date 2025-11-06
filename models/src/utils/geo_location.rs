use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Represents a geo location. Used to identify where a user logged in from,
/// etc (for audit log purposes).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd, TS)]
pub struct GeoLocation {
	/// The latitude of the location.
	pub latitude: f64,
	/// The longitude of the location.
	pub longitude: f64,
}
