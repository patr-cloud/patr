use std::fmt;

use api_macros::query;
use api_models::utils::Uuid;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::query_as;

#[derive(
	Debug, sqlx::Type, Serialize, Deserialize, PartialEq, Eq, Hash, Clone,
)]
#[sqlx(type_name = "DEPLOYMENT_PLAN", rename_all = "lowercase")]
pub enum DeploymentPlan {
	Free,
	Pro,
}

impl fmt::Display for DeploymentPlan {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			DeploymentPlan::Free => write!(f, "free"),
			DeploymentPlan::Pro => write!(f, "pro"),
		}
	}
}

#[derive(
	Debug, sqlx::Type, Serialize, Deserialize, PartialEq, Eq, Hash, Clone,
)]
#[sqlx(type_name = "DATABASE_PLAN", rename_all = "lowercase")]
pub enum DatabasePlan {
	Free,
	Unlimited,
}

impl fmt::Display for DatabasePlan {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			DatabasePlan::Free => write!(f, "free"),
			DatabasePlan::Unlimited => write!(f, "unlimited"),
		}
	}
}

#[derive(
	Debug, sqlx::Type, Serialize, Deserialize, PartialEq, Eq, Hash, Clone,
)]
#[sqlx(type_name = "SECRETS_PLAN", rename_all = "lowercase")]
pub enum SecretsPlan {
	Free,
	Pro,
	Unlimited,
}

impl fmt::Display for SecretsPlan {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			SecretsPlan::Free => write!(f, "free"),
			SecretsPlan::Pro => write!(f, "pro"),
			SecretsPlan::Unlimited => write!(f, "unlimited"),
		}
	}
}

#[derive(
	Debug, sqlx::Type, Serialize, Deserialize, PartialEq, Eq, Hash, Clone,
)]
#[sqlx(type_name = "DOCKER_REPO_PLAN", rename_all = "lowercase")]
pub enum DockerRepoPlan {
	Free,
	Pro,
	Unlimited,
}

impl fmt::Display for DockerRepoPlan {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			DockerRepoPlan::Free => write!(f, "free"),
			DockerRepoPlan::Pro => write!(f, "pro"),
			DockerRepoPlan::Unlimited => write!(f, "unlimited"),
		}
	}
}

pub async fn initialize_pricing_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Initializing pricing tables");

	// TODO - Come up with better enum names for resource pricing plans

	query!(
		r#"
		CREATE TYPE DEPLOYMENT_PLAN AS ENUM(
			'free',
			'pro', // Have to give it a better name
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DATABASE_PLAN AS ENUM(
			'free',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE SECRETS_PLAN AS ENUM(
			'free',
			'pro',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TYPE DOCKER_REPO_PLAN AS ENUM(
			'free',
			'pro',
			'unlimited'
		);
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE deployment_pricing
			id UUID CONSTRAINT deployment_pricing_pk PRIMARY KEY,
			plan TEXT NOT NULL,
			machine_type_id UUID NOT NULL CONSTRAINT
				deployment_pricing_fk_machine_type
					REFERENCES deployment_machine_type(id),
			number_of_deployments INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			min_replica INT NOT NULL,
			max_replica INT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE static_site_pricing
			id UUID CONSTRAINT static_site_pricing_pk PRIMARY KEY,
			plan TEXT NOT NULL,
			number_of_sites INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE domain_pricing
			id UUID CONSTRAINT domain_pricing_pk PRIMARY KEY,
			plan TEXT NOT NULL,
			number_of_domains INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_url_pricing
			id UUID CONSTRAINT managed_url_pricing_pk PRIMARY KEY,
			plan TEXT NOT NULL,
			number_of_urls INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE managed_database_pricing
			id UUID CONSTRAINT managed_database_pricing_pk PRIMARY KEY,
			plan TEXT NOT NULL,
			managed_database_machine_plan TEXT NOT NULL,
			number_of_databases INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE secret_pricing
			id UUID CONSTRAINT secrets_pricing_pk PRIMARY KEY,
			plan TEXT NOT NULL,
			number_of_secrets INT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			start_time TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	query!(
		r#"
		CREATE TABLE docker_repository_pricing
			id UUID NOT NULL PRIMARY KEY,
			plan TEXT NOT NULL,
			storage_in_byte BIGINT NOT NULL,
			price_in_cents BIGINT NOT NULL,
			start_time TIMESTAMPTZ NOT NULL,
			stop_time TIMESTAMPTZ;
		"#
	)
	.execute(&mut *connection)
	.await?;

	Ok(())
}

pub async fn initialize_billing_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> Result<(), sqlx::Error> {
	log::info!("Finishing up pricing tables initialization");

	Ok(())
}
