use crate::prelude::*;

/// Alert component. Used to display alerts.
#[component]
pub fn Alert(
	/// type of the alert
	#[prop(into)]
	r#type: MaybeSignal<NotificationType>,
	/// message to display
	#[prop(into)]
	message: MaybeSignal<String>,
	/// class name to apply to the alert
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	view! {
		<span class={move || format!("txt-white fr-fs-fs {}", class.get())}>
			<Icon
				icon={r#type
					.with(|r#type| {
						if matches!(r#type, NotificationType::Error | NotificationType::Warning) {
							IconType::AlertCircle
						} else {
							IconType::CheckCircle
						}
					})}

				size={Size::Small}
				color={r#type.with(NotificationType::as_patr_color)}
			/>
			<span class={move || {
				format!("ml-xxs {}", r#type.get().as_patr_color().as_text_color().as_css_color())
			}}>{message}</span>
		</span>
	}
}
