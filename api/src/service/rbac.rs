use std::collections::HashMap;

use api_models::utils::Uuid;
use eve_rs::AsError;
use serde_json::Value;

use crate::{error, utils::Error};

pub async fn extract_role_permissions(
	resource_permissions_body: Option<&Value>,
	resource_type_permissions_body: Option<&Value>,
) -> Result<(HashMap<Uuid, Vec<Uuid>>, HashMap<Uuid, Vec<Uuid>>), Error> {
	let resource_permissions_map =
		if let Some(Value::Object(permissions)) = resource_permissions_body {
			permissions
		} else {
			return Error::as_result()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string());
		};
	let resource_type_permissions_map = if let Some(Value::Object(
		permissions,
	)) = resource_type_permissions_body
	{
		permissions
	} else {
		return Error::as_result()
			.status(400)
			.body(error!(WRONG_PARAMETERS).to_string());
	};

	let mut resource_permissions = HashMap::new();
	let mut resource_type_permissions = HashMap::new();

	for (resource_id, permissions) in resource_permissions_map {
		let resource_id = if let Ok(resource_id) = Uuid::parse_str(resource_id)
		{
			resource_id
		} else {
			return Error::as_result()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string());
		};
		let permissions = if let Value::Array(permissions) = permissions {
			permissions
		} else {
			return Error::as_result()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string());
		};
		let mut permissions_values = Vec::with_capacity(permissions.len());
		for permission_id in permissions {
			let permission_id = if let Value::String(permission) = permission_id
			{
				permission
			} else {
				return Error::as_result()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string());
			};
			if let Ok(permission_id) = Uuid::parse_str(permission_id) {
				permissions_values.push(permission_id);
			} else {
				return Error::as_result()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string());
			}
		}
		resource_permissions.insert(resource_id, permissions_values);
	}
	for (resource_type_id, permissions) in resource_type_permissions_map {
		let resource_type_id =
			if let Ok(resource_type_id) = Uuid::parse_str(resource_type_id) {
				resource_type_id
			} else {
				return Error::as_result()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string());
			};
		let permissions = if let Value::Array(permissions) = permissions {
			permissions
		} else {
			return Error::as_result()
				.status(400)
				.body(error!(WRONG_PARAMETERS).to_string());
		};
		let mut permissions_values = Vec::with_capacity(permissions.len());
		for permission_id in permissions {
			let permission_id = if let Value::String(permission) = permission_id
			{
				permission
			} else {
				return Error::as_result()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string());
			};
			if let Ok(permission_id) = Uuid::parse_str(permission_id) {
				permissions_values.push(permission_id);
			} else {
				return Error::as_result()
					.status(400)
					.body(error!(WRONG_PARAMETERS).to_string());
			}
		}
		resource_type_permissions.insert(resource_type_id, permissions_values);
	}
	Ok((resource_permissions, resource_type_permissions))
}
