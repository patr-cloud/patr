use std::future::Future;

use inquire::{Select, Text};

use crate::prelude::*;

/// A reusable prompt widget that composes `inquire::Text` + `inquire::Select`
/// for search-based selection. The user types a search query, results are
/// fetched via an async function, and the user picks from the results or
/// searches again.
pub struct SearchAndSelect<'a, T, F, Fut>
where
	F: Fn(&str) -> Fut,
	Fut: Future<Output = Result<Vec<T>, AppError>>,
{
	message: &'a str,
	search_fn: F,
	display_fn: fn(&T) -> String,
	help_message: Option<&'a str>,
	page_size: usize,
}

impl<'a, T, F, Fut> SearchAndSelect<'a, T, F, Fut>
where
	F: Fn(&str) -> Fut,
	Fut: Future<Output = Result<Vec<T>, AppError>>,
{
	/// Create a new `SearchAndSelect` prompt with a message, async search
	/// function, and a display function for rendering items.
	pub fn new(message: &'a str, search_fn: F, display_fn: fn(&T) -> String) -> Self {
		Self {
			message,
			search_fn,
			display_fn,
			help_message: None,
			page_size: 10,
		}
	}

	/// Set a help message shown during the search text input.
	pub fn with_help_message(mut self, help_message: &'a str) -> Self {
		self.help_message = Some(help_message);
		self
	}

	/// Set the page size for the selection list.
	pub fn with_page_size(mut self, page_size: usize) -> Self {
		self.page_size = page_size;
		self
	}

	/// Run the search-and-select prompt loop. Returns `Ok(Some(item))` on
	/// selection, `Ok(None)` if the user cancels (Escape during text input).
	pub async fn prompt_skippable(self) -> Result<Option<T>, AppError> {
		loop {
			// First show the search prompt
			let mut text_prompt = Text::new(self.message);
			if let Some(help) = self.help_message {
				text_prompt = text_prompt.with_help_message(help);
			}

			let Some(query) = text_prompt
				.prompt_skippable()
				.expect_tty("Failed to read search query")
			else {
				return Ok(None);
			};

			// When they're pressed enter, show a spinner and trigger the search
			let results = (self.search_fn)(&query).await?;

			if results.is_empty() {
				eprintln!("No results found. Try a different search.");
				continue;
			}

			let display_items = std::iter::once("<< Search again".to_string())
				.chain(results.iter().map(self.display_fn))
				.collect::<Vec<_>>();

			// Show the results, with a "back" button
			let Some(selection) = Select::new("Select:", display_items)
				.with_page_size(self.page_size)
				.prompt_skippable()
				.expect_tty("Failed to read selection")
			else {
				continue;
			};

			// If they selected "back", loop again
			if selection == "<< Search again" {
				continue;
			}

			// They have selected an item
			let index = results
				.iter()
				.position(|item| (self.display_fn)(item) == selection)
				.expect("Selected item not found in results");

			return Ok(Some(
				results.into_iter().nth(index).expect("Index out of bounds"),
			));
		}
	}

	/// Run the search-and-select prompt loop. Returns `Ok(item)` on selection.
	/// Escape on either sub-prompt loops back to the search step.
	pub async fn prompt(self) -> Result<T, AppError> {
		loop {
			let mut text_prompt = Text::new(self.message);
			if let Some(help) = self.help_message {
				text_prompt = text_prompt.with_help_message(help);
			}

			let query = text_prompt
				.prompt()
				.expect_tty("Failed to read search query");

			let results = (self.search_fn)(&query).await?;

			if results.is_empty() {
				eprintln!("No results found. Try a different search.");
				continue;
			}

			let display_items: Vec<String> = std::iter::once("<< Search again".to_string())
				.chain(results.iter().map(self.display_fn))
				.collect();

			let Some(selection) = Select::new("Select:", display_items)
				.with_page_size(self.page_size)
				.prompt_skippable()
				.expect_tty("Failed to read selection")
			else {
				continue;
			};

			if selection == "<< Search again" {
				continue;
			}

			let index = results
				.iter()
				.position(|item| (self.display_fn)(item) == selection)
				.expect("Selected item not found in results");

			return Ok(results.into_iter().nth(index).expect("Index out of bounds"));
		}
	}
}
