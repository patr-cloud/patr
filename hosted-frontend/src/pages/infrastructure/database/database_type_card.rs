use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DatabaseType {
	MongoDB,
	Redis,
	Postgres,
	MySQL,
}

impl DatabaseType {
	pub const fn icon_link(&self) -> &'static str {
		match self {
			Self::MongoDB => "/icons/mongo.svg",
			Self::Redis => "/icons/redis.svg",
			Self::Postgres => "/icons/postgres.svg",
			Self::MySQL => "/icons/mysql.svg",
		}
	}

	pub const fn as_name_string(&self) -> &'static str {
		match self {
			Self::MongoDB => "MongoDB",
			Self::Redis => "Redis",
			Self::Postgres => "PostgresQL",
			Self::MySQL => "MySQL",
		}
	}
}

#[component]
pub fn DatabaseTypeCard(
	/// The Type of database
	database_type: DatabaseType,
	/// The Version number
	version: f64,
) -> impl IntoView {
	view! {
		<div class="fc-ct-ct bg-secondary-light br-sm px-md py-sm outline-info-focus database-type-card txt-white txt-sm">
			<img
				alt=database_type.as_name_string()
				src=database_type.icon_link()
				class="txt-grey txt-xxs"
			/>
			{move || database_type.as_name_string()}
			<small>{format!("Version {}", version)}</small>
		</div>
	}
}
