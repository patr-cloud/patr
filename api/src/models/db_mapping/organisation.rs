pub struct Organisation {
	pub id: Vec<u8>,
	pub name: String,
	pub super_admin_id: Vec<u8>,
	pub active: bool,
	pub created: u64,
}

pub struct Domain {
	pub id: Vec<u8>,
	pub name: String,
}

pub struct OrganisationDomain {
	pub id: Vec<u8>,
	pub is_verified: bool,
}

pub struct PersonalDomain {
	pub id: Vec<u8>,
}

// the following structs are for querying the db after initialisation
// Name can be changed for this struct because it is used in multiple queries
pub struct VerifiedDomains {
	pub id: Vec<u8>,
	pub name: String,
	pub is_verified: bool,
}

pub struct DomainsForOrganisation {
	pub name: String,
	pub is_verified: bool,
}

pub struct Application {
	pub id: Vec<u8>,
	pub name: String,
}

// struct to store information regarding the version for an application.
pub struct ApplicationVersion {
	pub application_id: Vec<u8>,
	pub version: String,
}
