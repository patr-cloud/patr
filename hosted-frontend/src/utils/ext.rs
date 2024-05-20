/// String extensions for convienence
pub trait StringExt {
	/// Wraps the string into an option depending on whether it's empty
	/// Returns None if string is empty otherwise returns the string wrapped in
	/// a `Some()`
	fn some_if_not_empty(self) -> Option<String>;
}

impl StringExt for String {
	fn some_if_not_empty(self) -> Option<String> {
		if self.is_empty() {
			None
		} else {
			Some(self)
		}
	}
}
