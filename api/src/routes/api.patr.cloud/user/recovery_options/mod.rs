mod update_user_email;
mod update_user_phone_number;
mod verify_user_email;
mod verify_user_phone_number;

use axum::Router;

pub use self::{
	update_user_email::*,
	update_user_phone_number::*,
	verify_user_email::*,
	verify_user_phone_number::*,
};
use crate::prelude::*;

/// Sets up the recovery options routes
#[instrument(skip(state))]
pub async fn setup_routes(state: &AppState) -> Router {
	Router::new()
		.mount_auth_endpoint(update_user_email, state)
		.mount_auth_endpoint(update_user_phone_number, state)
		.mount_auth_endpoint(verify_user_email, state)
		.mount_auth_endpoint(verify_user_phone_number, state)
}
