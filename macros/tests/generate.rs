use macros::generate_optional;

#[generate_optional]
pub struct User {
	pub name: String,
	pub age: u32,
}
