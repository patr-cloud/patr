use crate::prelude::*;

/// All sign up related data of a user
mod sign_up;
/// The main user data tables
mod user_data;
/// The user email tables
mod user_email;
/// The user login tables. This is used to store the login information of the
/// user and their API tokens.
mod user_login;
/// The phone numbers of the user
mod user_phone;

/// Initializes all user tables
#[instrument(skip(connection))]
pub async fn initialize_user_tables(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user tables");
	user_data::initialize_user_data_tables(&mut *connection).await?;
	user_email::initialize_user_email_tables(&mut *connection).await?;
	user_phone::initialize_user_phone_tables(&mut *connection).await?;
	user_login::initialize_user_login_tables(&mut *connection).await?;
	sign_up::initialize_user_sign_up_tables(&mut *connection).await?;

	Ok(())
}

/// Initializes all user indices
#[instrument(skip(connection))]
pub async fn initialize_user_indices(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user indices");
	user_data::initialize_user_data_indices(&mut *connection).await?;
	user_email::initialize_user_email_indices(&mut *connection).await?;
	user_phone::initialize_user_phone_indices(&mut *connection).await?;
	user_login::initialize_user_login_indices(&mut *connection).await?;
	sign_up::initialize_user_sign_up_indices(&mut *connection).await?;

	Ok(())
}

/// Initializes all user constraints
#[instrument(skip(connection))]
pub async fn initialize_user_constraints(
	connection: &mut DatabaseConnection,
) -> Result<(), sqlx::Error> {
	info!("Setting up user constraints");
	user_data::initialize_user_data_constraints(&mut *connection).await?;
	user_email::initialize_user_email_constraints(&mut *connection).await?;
	user_phone::initialize_user_phone_constraints(&mut *connection).await?;
	user_login::initialize_user_login_constraints(&mut *connection).await?;
	sign_up::initialize_user_sign_up_constraints(&mut *connection).await?;

	Ok(())
}
