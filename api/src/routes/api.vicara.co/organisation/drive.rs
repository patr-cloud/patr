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
use serde_json::Value;

pub fn create_sub_app(app: &App) -> EveApp<EveContext, EveMiddleware, App> {
	let mut sub_app = create_eve_app(app);

	// FILES
	// create a file
	sub_app.post(
		"/file/",
		&[
			EveMiddleware::ResourceTokenAuthenticator(
				permissions::organisation::drive::ADD,
				api_macros::closure_as_pinned_box!(|mut context| {
					let org_id =
						context.get_param(request_keys::ORGANISATION_ID);

					if org_id.is_none() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}

					let org_id = org_id.unwrap();
					let organisation_id = hex::decode(&org_id);
					if organisation_id.is_err() {
						context.status(400).json(error!(WRONG_PARAMETERS));
						return Ok((context, None));
					}
					let organisation_id = organisation_id.unwrap();

					let resource = db::get_resource_by_id(
						context.get_mysql_connection(),
						&organisation_id,
					)
					.await?;
					if resource.is_none() {
						context
							.status(404)
							.json(error!(RESOURCE_DOES_NOT_EXIST));
					}

					Ok((context, resource))
				}),
			),
			EveMiddleware::CustomFunction(pin_fn!(create_file)),
		],
	);

	// list all the files
	sub_app.get(
		"file/",
		&[EveMiddleware::CustomFunction(pin_fn!(list_files))],
	);

	// get file info9
	sub_app.get(
		"file/:fileId",
		&[EveMiddleware::CustomFunction(pin_fn!(get_file_info))],
	);

	sub_app
}

async fn create_file(
	mut context: EveContext,
	_: NextHandler<EveContext>,
) -> Result<EveContext, Error<EveContext>> {
	log::debug!("Here inside the middleware");
	let body = context.get_body_object().clone();

	let file_name = if let Some(Value::String(file_name)) =
		body.get(request_keys::FILE_NAME)
	{
		file_name
	} else {
		context.status(400).json(error!(WRONG_PARAMETERS));
		return Ok(context);
	};

	let folder_id: Option<&str> = match body.get(request_keys::FOLDER_ID) {
		Some(Value::String(folder_id)) => Some(folder_id),
		None => None,
		_ => {
			context.status(400).json(error!(WRONG_PARAMETERS));
			return Ok(context);
		}
	};

	let collection_id: Option<&str> =
		match body.get(request_keys::COLLECTION_ID) {
			Some(Value::String(collection_id)) => Some(collection_id),
			None => None,
			_ => {
				context.status(400).json(error!(WRONG_PARAMETERS));
				return Ok(context);
			}
		};

	let created = get_current_time();

	// upload file under the user-id folder.
	// if the user is part of an Organisation, then add the file under the organisation
	// save file details to the database

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
