use crate::imports::*;

/// A Single Log Item containing the timestamp and the log message
pub struct LogItem {
	/// The Timestamp of the log
	pub timestamp: String,
	/// The Log message
	pub log: String,
}

#[component]
pub fn LogStatement(
	/// Additional class names to apply to the outer div, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// The Log Details
	log: LogItem,
) -> impl IntoView {
	let outer_div_class = class.with(|cname| {
		format!(
			"text-grey log-statement w-full flex justify-start items-center full-width {}",
			cname
		)
	});

	view! {
		<div class={outer_div_class}>
			<Icon icon={IconType::ChevronRight} size={Size::ExtraSmall} color={Color::Grey}/>

			<ToolTipContainer
				tooltip_width=10.
				label={view! {
					<time class="w-[10ch]">
						// {getTimeFromNow(Date.parse(log.timestamp))}
						"13:00"
					</time>
				}
					.into_view()}
			>
				<time date_time="2008-02-14 20:00" class="text-xxs">
					// {new Date(log.timestamp).toUTCString()}
					"12:00"
				</time>
			</ToolTipContainer>
			" - "
			<span class="px-sm">{log.log}</span>
		</div>
	}
}
