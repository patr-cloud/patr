use serde::{Deserialize, Serialize};

/// Represents a geo location. Used to identify where a user logged in from,
/// etc (for audit log purposes).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, PartialOrd)]
pub struct GeoLocation {
	/// The latitude of the location.
	pub latitude: f64,
	/// The longitude of the location.
	pub longitude: f64,
}
