use crate::{
	app::{create_eve_app, App},
	utils::{EveContext, EveMiddleware},
};

use express_rs::{App as EveApp, Context};
use serde_json::json;

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(app);

	app.get(
		"/sign-in",
		&[EveMiddleware::CustomFunction(|mut context, _| {
			Box::pin(async move {
				context.json(json!({
					"success": true
				}));
				Ok(context)
			})
		})],
	);

	app
}
