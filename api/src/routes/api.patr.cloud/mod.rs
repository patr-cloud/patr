use axum::Router;

use crate::prelude::*;

pub fn setup_routes(state: &AppState) -> Router {
	Router::new().with_state(state.clone())
}
