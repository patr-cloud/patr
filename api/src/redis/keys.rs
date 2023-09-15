use crate::prelude::*;

/// The key used to store the permissions for a login ID
pub fn permission_for_login_id(login_id: &Uuid) -> String {
	format!("permission:{}", login_id)
}
