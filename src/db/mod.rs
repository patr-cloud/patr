mod initializer;
mod meta_data;
mod rbac;
mod user;
mod organisation;

use crate::utils::settings::Settings;
use sqlx::mysql::MySqlPool;

pub use initializer::initialize;
pub use meta_data::*;
pub use rbac::*;
pub use user::*;
pub use organisation::*;

pub async fn create_connection_pool(config: &Settings) -> Result<MySqlPool, sqlx::Error> {
	log::trace!("Creating database connection pool...");
	MySqlPool::builder()
		.max_size(config.mysql.connection_limit)
		.build(&format!(
			"mysql://{}:{}@{}:{}/{}",
			config.mysql.user,
			config.mysql.password,
			config.mysql.host,
			config.mysql.port,
			config.mysql.database
		))
		.await
}
