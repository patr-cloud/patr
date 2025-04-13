use crate::prelude::*;

/// Alert Component, used to show inline alert in forms and such,
/// e.g., if the user doesn't fill the username while logging in
#[component]
pub fn Alert(
	/// The Type of Alert
	r#type: AlertType,
	/// Additional Classes
	#[prop(into, optional)]
	class: Signal<String>,
	/// The Message
	children: Children,
) -> impl IntoView {
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

	let outer_class = move || {
		format!(
			"flex flex-row items-start justify-start text-white {}",
			class.get()
		)
	};

	view! {
		<span
			class={outer_class}
		>
			{match r#type {
				AlertType::Success => {
					view! {
						<Icon
							size={Size::Small}
							icon={IconType::CheckCircle}
							color={Color::Success}
						/>
					}
				}
				AlertType::Warning => {
					view! {
						<Icon
							size={Size::Small}
							icon={IconType::AlertCircle}
							color={Color::Warning}
						/>
					}
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
