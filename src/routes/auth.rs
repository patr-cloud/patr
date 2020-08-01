use crate::app::create_thruster_app;

use express_rs::{App as ThrusterApp, DefaultContext, DefaultMiddleware};

pub fn create_sub_app() -> ThrusterApp<DefaultContext, DefaultMiddleware> {
	let mut app = create_thruster_app();

	app.get(
		"/sign-in",
		&[DefaultMiddleware::new(|context, next| {
			Box::pin(async move { next(context).await })
		})],
	);

	app
}
