mod aws;
#[allow(clippy::module_inception)]
mod deployment;
mod digitalocean;
mod kubernetes;
mod managed_database;
mod static_site;

use std::ops::DerefMut;

use api_models::{
	models::workspace::infrastructure::deployment::DeploymentStatus,
	utils::Uuid,
};
use eve_rs::AsError;
use futures::StreamExt;
use shiplift::{Docker, PullOptions, RegistryAuth, TagOptions};

pub use self::{
	deployment::*,
	digitalocean::*,
	kubernetes::*,
	managed_database::*,
	static_site::*,
};
use crate::{
	db,
	models::{
		db_mapping::ManagedDatabaseStatus,
		rbac,
		RegistryToken,
		RegistryTokenAccess,
	},
	service,
	utils::{get_current_time, settings::Settings, Error},
};

async fn delete_docker_image(image_name: &str) -> Result<(), Error> {
	let docker = Docker::new();

	docker.images().get(image_name).delete().await?;

	Ok(())
}

pub(super) async fn tag_docker_image(
	image_id: &str,
	new_repo_name: &str,
) -> Result<(), Error> {
	let docker = Docker::new();
	docker
		.images()
		.get(image_id)
		.tag(
			&TagOptions::builder()
				.repo(new_repo_name)
				.tag("latest")
				.build(),
		)
		.await?;

	Ok(())
}

pub(super) async fn pull_image_from_registry(
	image_id: &str,
	config: &Settings,
) -> Result<(), Error> {
	let app = service::get_app().clone();
	let god_username = db::get_user_by_user_id(
		app.database.acquire().await?.deref_mut(),
		rbac::GOD_USER_ID.get().unwrap(),
	)
	.await?
	.status(500)?
	.username;

	let repo_name = image_id
		.replace(&format!("{}/", config.docker_registry.registry_url), "");
	let repo_name = if let Some(index) = repo_name.rfind("@sha") {
		repo_name[..index].to_string()
	} else {
		repo_name
	};

	// generate token as password
	let iat = get_current_time().as_secs();
	let token = RegistryToken::new(
		config.docker_registry.issuer.clone(),
		iat,
		god_username.clone(),
		config,
		vec![RegistryTokenAccess {
			r#type: "repository".to_string(),
			name: repo_name,
			actions: vec!["pull".to_string()],
		}],
	)
	.to_string(
		config.docker_registry.private_key.as_ref(),
		config.docker_registry.public_key_der.as_ref(),
	)?;

	// get token object using the above token string
	let registry_auth = RegistryAuth::builder()
		.username(god_username)
		.password(token)
		.build();

	let docker = Docker::new();
	let mut stream = docker.images().pull(
		&PullOptions::builder()
			.image(image_id)
			.auth(registry_auth)
			.build(),
	);

	while stream.next().await.is_some() {}

	Ok(())
}

async fn update_managed_database_status(
	database_id: &Uuid,
	status: &ManagedDatabaseStatus,
) -> Result<(), sqlx::Error> {
	let app = service::get_app();

	db::update_managed_database_status(
		app.database.acquire().await?.deref_mut(),
		database_id,
		status,
	)
	.await?;

	Ok(())
}

async fn update_managed_database_credentials_for_database(
	database_id: &Uuid,
	host: &str,
	port: i32,
	username: &str,
	password: &str,
) -> Result<(), sqlx::Error> {
	let app = service::get_app();

	db::update_managed_database_credentials_for_database(
		app.database.acquire().await?.deref_mut(),
		database_id,
		host,
		port,
		username,
		password,
	)
	.await?;

	Ok(())
}

async fn update_static_site_status(
	static_site_id: &Uuid,
	status: &DeploymentStatus,
) -> Result<(), Error> {
	let app = service::get_app();
	db::update_static_site_status(
		app.database.acquire().await?.deref_mut(),
		static_site_id,
		status,
	)
	.await?;

	Ok(())
}
