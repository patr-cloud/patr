use crate::{
	app::{create_eve_app, App},
	pin_fn,
	routes::api_bytesonus_com::middlewares::token_authenticator,
	utils::{EveContext, EveMiddleware},
};

use eve_rs::{App as EveApp, Error, NextHandler};

pub fn create_sub_app(app: App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut app = create_eve_app(app);

	app.get(
		"/info",
		&[
			EveMiddleware::CustomFunction(token_authenticator()),
			EveMiddleware::CustomFunction(pin_fn!(user_info)),
		],
	);

	app
}

async fn user_info(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	//println!("{:#?}", context.get_token_data());
	Ok(context)
}
