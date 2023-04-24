use crate::prelude::*;

mod sign_up;
mod user_data;
mod user_email;
mod user_login;
mod user_phone;

pub use self::{
	sign_up::*,
	user_data::*,
	user_email::*,
	user_login::*,
	user_phone::*,
};

pub async fn initialize_users_pre(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> DatabaseResult<()> {
	log::info!("Initializing user tables");
	user_data::initialize_user_data_pre(&mut *connection).await?;
	user_email::initialize_user_email_pre(&mut *connection).await?;
	user_phone::initialize_user_phone_pre(&mut *connection).await?;
	user_login::initialize_user_login_pre(&mut *connection).await?;
	sign_up::initialize_user_sign_up_pre(&mut *connection).await?;

	Ok(())
}

pub async fn initialize_users_post(
	connection: &mut <Database as sqlx::Database>::Connection,
) -> DatabaseResult<()> {
	log::info!("Finishing up user tables initialization");
	user_data::initialize_user_data_post(&mut *connection).await?;
	user_email::initialize_user_email_post(&mut *connection).await?;
	user_phone::initialize_user_phone_post(&mut *connection).await?;
	user_login::initialize_user_login_post(&mut *connection).await?;
	sign_up::initialize_user_sign_up_post(&mut *connection).await?;

	Ok(())
}
