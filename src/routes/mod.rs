mod auth;

use crate::{
	app::{create_thruster_app, App},
	utils::thruster_helpers::ThrusterContext,
};

use thruster::{
	App as ThrusterApp, Request,
};

pub fn create_sub_app(app: App) -> ThrusterApp<Request, ThrusterContext, App> {
	let mut sub_app = create_thruster_app(app.clone());

	sub_app.use_sub_app(
		"/auth",
		auth::create_sub_app(app),
	);

	sub_app
}
