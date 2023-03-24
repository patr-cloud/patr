mod deployment;
mod managedurl;
mod staticsite;

use api_models::utils::{ResourceType, Uuid};
use chrono::Utc;

use crate::{
	app::App,
	db::{
		self,
		add_personal_email_for_user,
		add_to_personal_domain,
		begin_deferred_constraints,
		create_resource,
		create_user,
		create_workspace,
		end_deferred_constraints,
		PaymentType,
		UserToSignUp,
	},
	models::rbac,
	rabbitmq,
	service::{self, create_stripe_customer},
	utils::{self, constants::default_limits, Error},
};

async fn init_tests() -> Result<App, Error> {
	let mut config = utils::settings::parse_config();
	config.database.database = "postgres".to_string();
	let database = db::create_database_connection(&config).await?;
	let mut connection = database.acquire().await?;
	let db_id = Uuid::new_v4();
	sqlx::query(&format!("CREATE DATABASE \"api_{}\"", db_id))
		.execute(&mut connection)
		.await?;

	config.database.database = format!("api_{}", db_id);
	let database = db::create_database_connection(&config).await?;
	let redis = crate::redis::create_redis_connection(&config).await?;
	let rabbitmq = rabbitmq::create_rabbitmq_pool(&config).await?;

	println!("database name: {}", config.database.database);
	let app = App {
		config,
		database,
		redis,
		rabbitmq,
	};
	service::initialize(&app);
	db::initialize(&app).await?;

	Ok(app)
}

async fn deinit_test(db_id: String) -> Result<(), Error> {
	let mut config = utils::settings::parse_config();
	config.database.database = "postgres".to_string();
	let database = db::create_database_connection(&config).await?;
	let mut connection = database.acquire().await?;

	sqlx::query(&dbg!(format!("DROP DATABASE {}  WITH (FORCE);", db_id)))
		.execute(&mut *connection)
		.await?;
	Ok(())
}

async fn user_constaints() -> Result<App, Error> {
	let config = utils::settings::parse_config();
	let app = init_tests().await.unwrap();
	let mut connection = app.database.begin().await?;
	let test_user_id = Uuid::new_v4();
	let test_user = UserToSignUp {
		username: "testuser".to_string(),
		account_type: ResourceType::Personal,
		password: "test".to_string(),
		first_name: "specimen".to_string(),
		last_name: "one".to_string(),
		recovery_email_local: Some("test12".to_string()),
		recovery_email_domain_id: Some(Uuid::new_v4()),
		recovery_phone_country_code: None,
		recovery_phone_number: None,
		business_email_local: None,
		business_domain_name: None,
		business_name: None,
		otp_hash: "0000".to_string(),
		otp_expiry: Utc::now(),
		coupon_code: None,
	};

	begin_deferred_constraints(&mut connection).await?;
	add_to_personal_domain(
		&mut connection,
		&test_user.recovery_email_domain_id.clone().unwrap(),
	)
	.await?;
	add_personal_email_for_user(
		&mut connection,
		&test_user_id,
		&test_user.recovery_email_local.clone().unwrap(),
		&test_user.recovery_email_domain_id.clone().unwrap(),
	)
	.await?;
	create_user(
		&mut connection,
		&test_user_id,
		&test_user.username,
		&test_user.password,
		(&test_user.first_name, &test_user.last_name),
		&Utc::now(),
		test_user.recovery_email_local.as_deref(),
		test_user.recovery_email_domain_id.as_ref(),
		test_user.recovery_phone_country_code.as_deref(),
		test_user.recovery_phone_number.as_deref(),
		3,
		test_user.coupon_code.as_deref(),
	)
	.await?;
	let resource_id = db::generate_new_resource_id(&mut connection).await?;
	let stripe_customer_id =
		create_stripe_customer(&resource_id, &config).await?;
	create_resource(
		&mut connection,
		&resource_id,
		rbac::RESOURCE_TYPES
			.get()
			.unwrap()
			.get(rbac::resource_types::WORKSPACE)
			.unwrap(),
		&resource_id,
		&Utc::now(),
	)
	.await?;
	create_workspace(
		&mut connection,
		&resource_id,
		"test-workspace",
		&test_user_id,
		&["test12@patr.cloud".to_string()],
		default_limits::DEPLOYMENTS,
		default_limits::MANAGED_DATABASE,
		default_limits::STATIC_SITES,
		default_limits::MANAGED_URLS,
		default_limits::DOCKER_REPOSITORY_STORAGE,
		default_limits::DOMAINS,
		default_limits::SECRETS,
		default_limits::VOLUME_STORAGE,
		&stripe_customer_id,
		&PaymentType::Card,
	)
	.await?;
	end_deferred_constraints(&mut connection).await?;
	connection.commit().await?;
	Ok(app)
}
