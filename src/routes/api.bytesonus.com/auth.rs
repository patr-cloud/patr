use crate::{
	app::{create_eve_app, App},
	utils::{EveContext, EveMiddleware}, pin_fn,
};

use express_rs::{App as EveApp, Context, Error, NextHandler};
use serde_json::json;

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(app);

	app.get(
		"/sign-in",
		&[EveMiddleware::CustomFunction(pin_fn!(sign_in))],
	);

	app
}

async fn sign_in(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	context.json(json!({
		"success": true
	}));
	Ok(context)
}
