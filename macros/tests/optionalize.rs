#![allow(missing_docs, clippy::missing_docs_in_private_items)]

use macros::optionalize;
use models::utils::Optionalizable;

#[optionalize]
pub struct User {
	pub name: String,
	pub age: u32,
}

#[optionalize]
pub struct Coordinates(pub i32, pub i32);

#[optionalize]
pub struct UnitStruct;

#[optionalize]
pub struct GenericPayload<T> {
	pub id: u64,
	pub payload: T,
}

#[optionalize]
pub struct UserWithSkippedField {
	pub name: String,
	#[optionalize(skip)]
	pub internal_id: u64,
	pub age: u32,
}

#[optionalize]
pub struct TupleWithSkippedField(pub i32, #[optionalize(skip)] pub i32, pub i32);

#[optionalize]
pub struct UsesGenerics {
	pub id: u64,
	pub payload: Option<String>,
}

#[optionalize]
pub struct UsesKeepAttribute {
	pub id: u64,
	#[optionalize(keep)]
	pub payload: Option<String>,
}

#[optionalize]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeriveCarrier {
	pub name: String,
}

#[test]
fn keeps_original_struct_and_generates_optional_named_struct() {
	let user = User {
		name: "patr".to_owned(),
		age: 7,
	};
	assert_eq!(user.name, "patr");
	assert_eq!(user.age, 7);

	let none_set = UserOptional {
		name: None,
		age: None,
	};
	assert!(!none_set.any_field_set());
	assert!(!none_set.all_fields_set());

	let one_set = UserOptional {
		name: Some("patr".to_owned()),
		age: None,
	};
	assert!(one_set.any_field_set());
	assert!(!one_set.all_fields_set());

	let all_set = UserOptional {
		name: Some("patr".to_owned()),
		age: Some(7),
	};
	assert!(all_set.any_field_set());
	assert!(all_set.all_fields_set());
}

#[test]
fn handles_tuple_structs() {
	let coords = Coordinates(1, 2);
	assert_eq!(coords.0, 1);
	assert_eq!(coords.1, 2);

	let none_set = CoordinatesOptional(None, None);
	assert!(!none_set.any_field_set());
	assert!(!none_set.all_fields_set());

	let partial = CoordinatesOptional(Some(1), None);
	assert!(partial.any_field_set());
	assert!(!partial.all_fields_set());

	let all = CoordinatesOptional(Some(1), Some(2));
	assert!(all.any_field_set());
	assert!(all.all_fields_set());
}

#[test]
fn handles_unit_structs() {
	let unit = UnitStruct;
	let _ = unit;

	let optional = UnitStructOptional;
	assert!(!optional.any_field_set());
	assert!(optional.all_fields_set());
}

#[test]
fn preserves_generics() {
	let optional = GenericPayloadOptional::<String> {
		id: Some(1),
		payload: Some("data".to_owned()),
	};
	assert!(optional.any_field_set());
	assert!(optional.all_fields_set());

	let none_set = GenericPayloadOptional::<String> {
		id: None,
		payload: None,
	};
	assert!(!none_set.any_field_set());
	assert!(!none_set.all_fields_set());
}

#[test]
fn skips_named_fields_in_generated_optional_struct() {
	let original = UserWithSkippedField {
		name: "patr".to_owned(),
		internal_id: 9,
		age: 7,
	};
	assert_eq!(original.internal_id, 9);

	let optional = UserWithSkippedFieldOptional {
		name: Some("patr".to_owned()),
		age: None,
	};
	assert!(optional.any_field_set());
	assert!(!optional.all_fields_set());

	let all_set = UserWithSkippedFieldOptional {
		name: Some("patr".to_owned()),
		age: Some(7),
	};
	assert!(all_set.any_field_set());
	assert!(all_set.all_fields_set());
}

#[test]
fn skips_tuple_fields_in_generated_optional_struct() {
	let original = TupleWithSkippedField(1, 2, 3);
	assert_eq!(original.1, 2);

	let none_set = TupleWithSkippedFieldOptional(None, None);
	assert!(!none_set.any_field_set());
	assert!(!none_set.all_fields_set());

	let partial = TupleWithSkippedFieldOptional(Some(1), None);
	assert!(partial.any_field_set());
	assert!(!partial.all_fields_set());

	let all_set = TupleWithSkippedFieldOptional(Some(1), Some(3));
	assert!(all_set.any_field_set());
	assert!(all_set.all_fields_set());
}

#[test]
fn handles_structs_with_generic_fields() {
	let optional = UsesGenericsOptional {
		id: Some(1),
		payload: Some(Some("data".to_owned())),
	};
	assert!(optional.any_field_set());
	assert!(optional.all_fields_set());
}

#[test]
fn keeps_optional_fields_unchanged_when_requested() {
	let optional = UsesKeepAttributeOptional {
		id: Some(1),
		payload: Some("data".to_owned()),
	};
	assert!(optional.any_field_set());
	assert!(optional.all_fields_set());

	let payload: Option<String> = optional.payload.clone();
	assert_eq!(payload, Some("data".to_owned()));
}

#[test]
fn implements_optionalizable_trait() {
	let value: <User as Optionalizable>::Optionalized = UserOptional {
		name: Some("patr".to_owned()),
		age: Some(7),
	};
	assert!(value.all_fields_set());
}

#[test]
fn inherits_parent_derives() {
	let value = DeriveCarrierOptional {
		name: Some("patr".to_owned()),
	};
	let cloned = value.clone();
	assert_eq!(cloned, value);

	let debug_text = format!("{cloned:?}");
	assert!(debug_text.contains("DeriveCarrierOptional"));
}
