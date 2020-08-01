
pub mod permissions {
	pub mod docker {
		pub const PUSH: &str = "docker::push";
		pub const PULL: &str = "docker::pull";
	}

	pub mod deployer {
		pub const DEPLOY: &str = "deployer::deploy";
	}
}
