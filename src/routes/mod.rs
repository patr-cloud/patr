mod auth;

use crate::app::create_thruster_app;
use express_rs::{App as ThrusterApp, DefaultContext, DefaultMiddleware};

pub fn create_sub_app() -> ThrusterApp<DefaultContext, DefaultMiddleware> {
	let mut sub_app = create_thruster_app();

	sub_app.use_sub_app("/auth", auth::create_sub_app());

	sub_app
}
