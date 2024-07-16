use convert_case::{Case, Casing};
use models::api::workspace::database::DatabaseEngine;

use crate::prelude::*;

/// Type of databases thant can be used
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DatabaseType {
	/// Mongo DB database
	MongoDB,
	/// Redis database
	Redis,
	/// PostgresQL Database
	Postgres,
	/// MySQL Database
	MySQL,
}

impl DatabaseType {
	/// Converts the database type to the icon asset link to be used for images
	pub const fn icon_link(&self) -> &'static str {
		match self {
			Self::MongoDB => "/icons/mongo.svg",
			Self::Redis => "/icons/redis.svg",
			Self::Postgres => "/icons/postgres.svg",
			Self::MySQL => "/icons/mysql.svg",
		}
	}

	/// The Title of the database type
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
	#[prop(into)]
	database_type: MaybeSignal<DatabaseEngine>,
	/// The Version number
	version: f64,
) -> impl IntoView {
	view! {
		<div class="fc-ct-ct bg-secondary-light br-sm px-md py-sm outline-info-focus database-type-card txt-white txt-sm">
			<img
				alt={database_type.get().to_string()}
				src={format!("/icons/{}.svg", database_type.get().clone())}
				class="txt-grey txt-xxs"
			/>
			{move || database_type.get().to_string().to_case(Case::Pascal)}
			<small>{format!("Version {}", version)}</small>
		</div>
	}
}
