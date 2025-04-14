use std::fmt::{self, Display, Formatter};

use crate::imports::*;

/// The type of alert to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AlertType {
	/// Show an error Alert, with a red background
	Error,
	/// Show a warning Alert, with a yellow background
	Warning,
	/// Show a success Alert, with a green background
	Success,
}

impl Display for AlertType {
	fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.as_css_name())
	}
}

impl AlertType {
	/// Returns the CSS name of the color.
	pub const fn as_css_name(self) -> &'static str {
		match self {
			Self::Error => "error",
			Self::Warning => "warning",
			Self::Success => "success",
		}
	}
}

#[component]
pub fn Alert(
	/// The Type of the elert
	#[prop(into)]
	r#type: AlertType,
	/// Additional classes to apply
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Message
	children: ChildrenFn,
) -> impl IntoView {
	let message_class = move || format!("ml-xxs text-{}", r#type);
	let outer_class = move || {
		format!(
			"flex flex-row items-start justify-start text-white {}",
			class.get()
		)
	};

	view! {
		<span class={outer_class}>
			{match r#type {
				AlertType::Success => {
					view! {
						<Icon
							size={Size::Small}
							icon={IconType::CheckCircle}
							color={Color::Success}
						/>
					}
						.into_view()
				}
				AlertType::Warning => {
					view! {
						<Icon
							size={Size::Small}
							icon={IconType::AlertCircle}
							color={Color::Warning}
						/>
					}
						.into_view()
				}
				AlertType::Error => {
					view! {
						<Icon size={Size::Small} icon={IconType::AlertCircle} color={Color::Error}/>
					}
				}
			}}
			<span class={message_class}>{children()}</span>
		</span>
	}
}
