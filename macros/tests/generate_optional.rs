#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use macros::generate_optional;

#[generate_optional]
pub struct User {
	pub name: String,
	pub age: u32,
}

#[generate_optional]
pub struct UnitStruct;
