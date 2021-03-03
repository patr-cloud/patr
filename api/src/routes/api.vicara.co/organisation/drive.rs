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

use s3::region::Region;
// use s3::S3Error;
use s3::{bucket::Bucket, creds::Credentials};
use std::fs;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncRead;
// use tokio::prelude::{AsyncRead, Future};

use eve_rs::{App as EveApp, Context, Error, NextHandler};
use serde_json::{json, Value};

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

	// get file info
	sub_app.get(
		"file/:fileId",
		&[EveMiddleware::CustomFunction(pin_fn!(get_file_info))],
	);

	sub_app
}

// {
// 	fileName,
// 	folderId,
// 	collectionId,
// }
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

	log::debug!(
		"Received s3 config details is {:#?}",
		&context.get_state().config.s3
	);

	// get s3 credentials.
	let ACCESS_KEY = &context.get_state().config.s3.key;
	let SECRET_KEY = &context.get_state().config.s3.secret;
	let BUCKET = &context.get_state().config.s3.bucket;
	let REGION = &context.get_state().config.s3.region;
	let S3_HOST = &context.get_state().config.s3.endpoint;

	// init credentials for s3
	let credentials =
		Credentials::new(Some(ACCESS_KEY), Some(SECRET_KEY), None, None, None);
	if let Err(err) = credentials {
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}
	let credentials = credentials.unwrap();

	let region = REGION.parse().unwrap();
	let bucket = Bucket::new(&BUCKET, region, credentials);
	if let Err(err) = bucket {
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}
	let bucket = bucket.unwrap();

	// init bucket

	// create a temp file, and upload it to s3

	// generate path to temp file
	let mut file =
		fs::write(format!("./temp/{}", &file_name), format!("{}", &file_name));

	if let Err(err) = file {
		log::error!("Error creating the file. Error {:?}", err);
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}
	let file = file.unwrap();

	let file = File::open(format!("./temp/{}", &file_name)).await?;

	let path_string = format!("./temp/{}", &file_name);
	let file_path = Path::new(&path_string);

	log::debug!("Here before status creation");
	// let status = bucket
	// 	.put_object_stream(file_path, format!("araciv-test/"))
	// 	.await;
	let status = bucket
		.put_object(path_string, b"this is my test file")
		.await;

	log::debug!("Here after status creation");
	if let Err(err) = status {
		context.status(500).json(error!(SERVER_ERROR));
		return Ok(context);
	}

	let status = status.unwrap();
	log::debug!("Status for upload is {:?}", &status);

	context.json(json!({
		request_keys::SUCCESS: true,
		request_keys::MESSAGE: "File created",
		"S3-status" : &status
	}));

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
