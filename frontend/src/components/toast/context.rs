use rand::Rng;

use crate::imports::*;

/// All Info regarding the Toast.
#[derive(Clone, Debug)]
pub struct ToastData {
	/// The ID of the toast
	pub id: u64,
	/// The Alert Level of the Toast
	pub level: AlertType,
	/// The Toast Expires after, set None so that the toast requires some action
	/// for it to expire
	pub expiry: Option<u32>,
	/// Whether You can dismiss the toast with a click
	pub dismissible: bool,
	/// The Message to show in the toast
	pub message: String,
	/// Clear the toast
	pub clear: RwSignal<bool>,
}

impl ToastData {
	/// Build a default ToastBuilder
	pub fn builder() -> ToastBuilder {
		ToastBuilder::new()
	}
}

/// Builder for ToastData
pub struct ToastBuilder {
	/// The Alert Level of the Toast
	pub level: AlertType,
	/// The Toast Expires after, in milliseconds, set None so that the toast
	/// requires some action for it to expire
	pub expiry: Option<u32>,
	/// Whether You can dismiss the toast with a click
	pub dismissible: bool,
	/// The Message to show in the toast
	pub message: String,
}

impl ToastBuilder {
	/// Constructs a new toast builder with the supplied message.
	pub fn new() -> Self {
		ToastBuilder {
			message: "".into(),
			level: AlertType::Warning,

			dismissible: true,
			expiry: Some(2_000),
		}
	}

	#[must_use]
	/// Sets the message of the toast
	pub fn message(mut self, message: &str) -> Self {
		self.message = message.to_string();
		self
	}

	/// Sets the level of the toast.
	pub fn level(mut self, level: AlertType) -> Self {
		self.level = level;
		self
	}

	/// Sets the dismissable flag of the toast to allow or disallow the toast
	/// from being dismissable on click.
	pub fn dismissible(mut self, dismissible: bool) -> Self {
		self.dismissible = dismissible;
		self
	}

	/// Sets the expiry time of the toast in milliseconds, or disables it on
	/// `None`.
	pub fn expiry(mut self, expiry: Option<u32>) -> Self {
		self.expiry = expiry;
		self
	}

	/// Build the toast data
	pub fn build(self, id: u64) -> ToastData {
		ToastData {
			id,
			level: self.level,
			expiry: self.expiry,
			dismissible: self.dismissible,
			message: self.message,
			clear: create_rw_signal(false),
		}
	}
}

/// This contains the queue for the toast.
#[derive(Clone, Debug)]
pub struct ToasterContext {
	/// The Queue which displays the toasts
	pub queue: RwSignal<Vec<ToastData>>,
}

impl Default for ToasterContext {
	/// The Default Context
	fn default() -> Self {
		ToasterContext {
			queue: create_rw_signal(Vec::new()),
		}
	}
}

impl ToasterContext {
	/// Add a toast to the queue.
	pub fn toast(&self, toast_builder: ToastBuilder) {
		let mut rng = rand::thread_rng();
		let toast_id = rng.gen::<u64>();

		let toast = toast_builder.build(toast_id);
		self.queue.update(|queue| {
			if queue.len() > 5 {
				queue.remove(0);
			};
			queue.push(toast);
		})
	}

	/// Clear all toasts.
	pub fn clear(&self) {
		for toast in &self.queue.get_untracked() {
			toast.clear.set(true);
		}
	}

	/// Removes a toast of given ID
	pub fn remove(&self, id: u64) {
		let index = self
			.queue
			.get_untracked()
			.iter()
			.enumerate()
			.find(|(_, toast)| toast.id == id)
			.map(|(index, _)| index);

		if let Some(index) = index {
			let mut queue = self.queue.get_untracked();
			queue.remove(index);

			self.queue.set(queue);
		}
	}
}

/// Provide the Toaster Context
pub fn provide_toaster() {
	if use_context::<ToasterContext>().is_none() {
		provide_context(ToasterContext::default());
	}
}

/// Get the Toaster Context
pub fn expect_toaster() -> ToasterContext {
	use_context::<ToasterContext>().expect("No ToasterContext found")
}
