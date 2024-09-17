use models::api::workspace::deployment::*;
use time::macros::format_description;

use crate::prelude::*;

/// The Log Statement component. The log statement component is used to display
/// a log statement.
#[component]
pub fn LogStatement(
	/// Additional class names to apply to the outer div, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Log Details
	log: Signal<DeploymentLogs>,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| {
		format!(
			"text-grey log-statement w-full flex justify-start items-center full-width {}",
			cname
		)
	});

	let store_log = store_value(log);
	let date_formater = format_description!("[year]-[month]-[day] [hour]:[minute]");

	view! {
		<div class={outer_div_class}>
			<Icon icon={IconType::ChevronRight} size={Size::ExtraSmall} color={Color::Grey}/>

			{
				move || match store_log.with_value(|log| log.get().timestamp.format(&date_formater)) {
					Ok(date_time) => view! {
						<time date_time={date_time.clone()} class="text-xxs pr-sm">
							{date_time.clone()}
						</time>
					}.into_view(),
					Err(_) => view! {}.into_view()
				}
			}
			<time date_time="2008-02-14 20:00" class="text-xxs pr-sm">
				// {new Date(log.timestamp).toUTCString()}
				{store_log.with_value(|log| log.get().timestamp.format(&date_formater))}
			</time>
			" - "
			<span class="px-sm">
				{store_log.with_value(|log| log.get().log.clone())}
			</span>
		</div>
	}
}
