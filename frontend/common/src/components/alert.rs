use crate::prelude::*;

/// Alert Component, used to show inline alert in forms and such,
/// e.g., if the user doesn't fill the username while logging in
#[component]
pub fn Alert(
	/// The Type of Alert
	r#type: AlertType,
	/// Additional Classes
	#[props(into)]
	class: Signal<String>,
	/// The Message
	#[props(into)]
	children: Children,
) -> Element {
	let message_class = move || {
		format!(
			"ml-xxs {}",
			match r#type {
				AlertType::Success => "text-success",
				AlertType::Error => "text-error",
				AlertType::Warning => "text-warning",
			}
		)
	};

	rsx! {
		span {
			class: move || format!(
				"flex flex-row items-start justify-start text-white {}",
				class.read()
			),
			{
				match r#type {
					AlertType::Success => {
						rsx! {
							Icon {
								size: Size::Small,
								icon: IconType::CheckCircle,
								color: Color::Success
							}
						}
					}
					AlertType::Warning => {
						rsx! {
							Icon {
								size: Size::Small,
								icon: IconType::AlertCircle,
								color: Color::Warning,
							}
						}
					}
					AlertType::Error => {
						rsx! {
							Icon {
								size: Size::Small,
								icon: IconType::AlertCircle,
								color: Color::Error,
							}
						}
					}
				}
				span {
					class: message_class,
					children()
				}
			}
		}
	}
}
