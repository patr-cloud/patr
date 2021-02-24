use crate::{
	app::{create_eve_app, App},
	db, error,
	models::rbac::{self, permissions},
	pin_fn,
	utils::{
		constants::{self, request_keys},
		get_current_time, EveContext, EveMiddleware,
	},
};

use eve_rs::{App as EveApp, Context, Error, NextHandler};

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut sub_app = create_eve_app(app);

	// create a file
	sub_app.post(
		"file/",
		&[EveMiddleware::CustomFunction(pin_fn!(create_file))],
	);

	// list all the files
	sub_app.get(
		"file/",
		&[EveMiddleware::CustomFunction(pin_fn!(list_files))],
	);

	// get file info
	sub_app.get(
		"file/:fileId",
		&[EveMiddleware::CustomFunction(pin_fn!(get_file_detail))],
	);

	sub_app
}

async fn create_file(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	Ok(context)
}

async fn list_files(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	Ok(context)
}

async fn get_file_info(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	Ok(context)
}
