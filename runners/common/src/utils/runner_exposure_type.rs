use std::net::IpAddr;

/// This enum represents how the Runner will expose the resources to the
/// outside world. This is used to determine how the Runner will handle the
/// resources, such as whether it will use a tunnel, or whether it will
/// expose the resources directly, or if each resource has it's own exposed URL
/// on it's own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerExposureType {
	/// The runner will need to expose the resources through a tunnel, and run a
	/// reverse proxy to the resources.
	Private,
	/// The runner has a public IP address, and the resources will be exposed
	/// through a reverse proxy. This runner will not expose the resources
	/// through a tunnel, but will run a reverse proxy to the resources.
	PublicIP {
		/// The public IP address(es) of the runner. This is what will be used
		/// as the DNS record.
		ip_addresses: Vec<IpAddr>,
	},
	/// The runner has a public DNS name, and the resources will be exposed
	/// through a reverse proxy. This runner will not expose the resources
	/// through a tunnel, but will run a reverse proxy to the resources.
	PublicDNS {
		/// The public DNS name of the runner. This is what will be used as the
		/// CNAME DNS record.
		dns_name: String,
	},
}

impl RunnerExposureType {
	/// Returns true if the runner is a private runner, meaning it will
	/// expose the resources through a tunnel, and run a reverse proxy to the
	/// resources.
	#[must_use]
	pub fn is_private(&self) -> bool {
		matches!(self, RunnerExposureType::Private)
	}

	/// Returns true if the runner is a public runner, meaning it has a public
	/// IP address or a public DNS name, and will expose the resources through
	/// a reverse proxy.
	#[must_use]
	pub fn is_public(&self) -> bool {
		matches!(
			self,
			RunnerExposureType::PublicIP { .. } | RunnerExposureType::PublicDNS { .. }
		)
	}

	/// Returns true if the runner needs to run a tunnel to expose the
	/// resources. This is true for private runners, which will run a tunnel
	/// to expose the resources, and false for public runners, which will not
	/// run a tunnel.
	#[must_use]
	pub fn requires_tunnel(&self) -> bool {
		matches!(self, RunnerExposureType::Private)
	}
}
