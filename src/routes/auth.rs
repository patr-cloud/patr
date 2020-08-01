use crate::{
	app::{create_thruster_app, App},
	utils::thruster_helpers::ThrusterContext,
};

use thruster::{async_middleware, App as ThrusterApp, Request};

pub fn create_sub_app(app: App) -> ThrusterApp<Request, ThrusterContext, App> {
	let mut app = create_thruster_app(app);

	app.get(
		"/sign-in",
		async_middleware!(
			ThrusterContext,
			[|context, next| { Box::pin(async move { next(context).await }) }]
		),
	);

	app
}
