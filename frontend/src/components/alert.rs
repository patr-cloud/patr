use std::borrow::Cow;

use crate::prelude::*;

/// Alert component. Used to display alerts.
#[component]
pub fn Alert<'a>(
	/// scope of the component
	cx: Scope,
	/// type of the alert
	r#type: NotificationType,
	/// message to display
	#[prop(into)]
	msg: Cow<'a, str>,
	/// class name to apply to the alert
	#[prop(into, optional)]
	class_name: Cow<'a, str>,
) -> impl IntoView {
	view! { cx,
		<span class={format!("txt-white fr-fs-fs {}", class_name)}>
			<Icon
				icon={if let Error | Warning = r#type {
					"alert-circle"
				} else {
					"check-circle"
				}}
				color={r#type.as_patr_color()}
			/>
			<span class={format!(
				"ml-xxs txt-{}", r#type.as_css_name()
			)}>{msg}</span>
		</span>
	}
}
