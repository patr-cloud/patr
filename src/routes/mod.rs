mod auth;

use crate::{
	app::{create_eve_app, App},
	utils::{EveContext, EveMiddleware},
};
use express_rs::{App as EveApp, Context};
use serde_json::json;

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut sub_app = create_eve_app(app.clone());

	sub_app.use_sub_app("/auth", auth::create_sub_app(app));
	sub_app.get(
		"/v1/applications/:applicationId/:version/:platform",
		&[EveMiddleware::CustomFunction(|mut context, _| {
			Box::pin(async move {
				context.json(json!({
					"success": true
				}));
				Ok(context)
			})
		})],
	);

	sub_app
}
