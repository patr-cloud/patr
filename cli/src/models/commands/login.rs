use tabled::settings::Style;

/// The table output of the Login command
#[derive(Debug, Clone)]
pub struct Table {
	/// The username of the logged in user.
	pub username: String,
	/// The first name of the logged in user.
	pub first_name: String,
	/// The last name of the logged in user.
	pub last_name: String,
}

impl Table {
	/// Convert the struct into a tabled::Table
	pub fn into_formatted(self) -> tabled::Table {
		let mut builder = tabled::Table::builder([format!(
			"Logged in as {}. Hello, {} {}!",
			self.username, self.first_name, self.last_name
		)]);
		builder.remove_header();
		let mut table = builder.build();
		table.with(Style::blank());
		table
	}
}
