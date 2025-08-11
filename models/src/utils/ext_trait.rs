use std::collections::HashSet;

use crate::iaac::*;

/// A helper trait to deduplicate Iaac resources based on their names.
/// This is useful to ensure that there are no duplicate resources in the Iaac
/// file, as this can lead to unexpected behavior during deployment.
pub trait DeduplicatedIaacResourceExt {
	/// Deduplicates the Iaac resources based on their name.
	fn deduplicated(self) -> Result<HashSet<IaacResource>, IaacError>;
}

impl DeduplicatedIaacResourceExt for Vec<IaacResource> {
	fn deduplicated(self) -> Result<HashSet<IaacResource>, IaacError> {
		let mut deduplicated = HashSet::with_capacity(self.len());

		for resource in self {
			let resource_type = resource.resource_type();
			let resource_name = resource.name().clone().resolve_value()?;
			if !deduplicated.insert(resource) {
				return Err(IaacError::DuplicateResource(format!(
					"Duplicate resource found: {resource_type} `{resource_name}`",
				)));
			}
		}

		Ok(deduplicated)
	}
}

/// A helper trait to order Iaac resources based on their dependencies.
/// This is useful to ensure that resources are created in the correct order,
/// respecting their dependencies.
pub trait OrderedIaacResourceExt {
	/// Orders the Iaac resources based on their dependencies. This is useful to
	/// ensure that resources are created in the correct order, respecting their
	/// dependencies.
	fn ordered(self) -> Result<Vec<IaacResource>, IaacError>;
}

impl OrderedIaacResourceExt for HashSet<IaacResource> {
	fn ordered(self) -> Result<Vec<IaacResource>, IaacError> {
		todo!()
	}
}
