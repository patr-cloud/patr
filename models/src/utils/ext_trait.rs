use std::collections::HashSet;

use either::Either;

use crate::{iaac::*, utils::TryIteratorExt};

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
	fn ordered(mut self) -> Result<Vec<IaacResource>, IaacError> {
		// Iterate through the resources, and for each iteration, add the
		// resources that have all their dependencies satisfied to the ordered
		// list. If no resources are added in an iteration, and there are
		// still resources left, it means there is a circular dependency or
		// unsatisfied dependencies.
		let mut ordered = Vec::with_capacity(self.len());

		loop {
			if self.is_empty() {
				break;
			}

			// For each remaining resource, check if all it's dependencies are satisfied.
			// Upon finding the first one that has it's dependencies satisfied, just pop
			// that and add it to the ordered list. If none are found, then there are zero
			// dependencies that can be satisfied for all resources left, meaning there is a
			// circular dependency or unsatisfied dependencies.
			let Some(resource) = TryIteratorExt::try_find(&mut self.iter(), |resource| {
				// For each resource, check if all it's dependencies are satisfied.
				resource.dependencies().iter().try_all(|dependency| {
					// For each dependency, check if it is satisfied.
					let resource_type_matches = dependency
						.resource
						.map(|resource_type| resource.data.get_resource_type() == resource_type)
						.unwrap_or(true);

					if !resource_type_matches {
						// The dependency resource type does not match, so this dependency
						// is not satisfied.
						return Ok(false);
					}

					let Either::Right(name) = dependency.identifier.clone() else {
						return Err(IaacError::Unsupported(
							"Specifying dependencies by ID is not supported yet".to_string(),
						));
					};

					ordered.iter().try_any(|res: &IaacResource| {
						Ok(res.name().clone().resolve_value()? == name)
					})
				})
			})?
			else {
				return Err(IaacError::ResourceDependencyNotSatisfied(
					self.iter()
						.map(|res| res.name().clone().resolve_value())
						.collect::<Result<Vec<_>, _>>()?,
				));
			};

			let resource = resource.clone();
			self.remove(&resource);
			ordered.push(resource);
		}

		Ok(ordered)
	}
}
