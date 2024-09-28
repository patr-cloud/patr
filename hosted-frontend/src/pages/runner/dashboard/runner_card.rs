use models::api::workspace::runner::Runner;

use crate::prelude::*;

#[component]
pub fn RunnerCard(
	/// The Runner Info
	#[prop(into)]
	runner: MaybeSignal<WithId<Runner>>,
) -> impl IntoView {
	view! {
		<div class="bg-secondary-light flex flex-col items-start justify-start px-lg py-md br-sm text-white gap-xs">
			<div class="w-full flex items-center justify-start gap-md">
				<p class="text-md text-primary text-ellipsis overflow-hidden">
					{runner.get().name.clone()}
				</p>

				{match runner.get().last_seen {
					Some(_) => {
						view! {
							<StatusBadge
								text={Some("unreachable".to_string())}
								color={Some(Color::Grey)}
							/>
						}
					}
					None => {
						view! {
							<StatusBadge
								text={Some("live".to_string())}
								color={Some(Color::Success)}
							/>
						}
							.into_view()
					}
				}}
				<StatusBadge />
			</div>

			<div class="flex-2 w-full gap-xs flex items-center justify-center">
				<div class="bg-secondary-medium br-sm px-lg py-sm flex flex-col items-start justify-center w-full">
					<small class="letter-sp-md text-xxs text-grey">"LAST SEEN"</small>
					<p class="text-primary w-[15ch] text-ellipsis overflow-hidden">
						{match runner.get().last_seen {
							Some(date) => date.to_string().into_view(),
							None => "Just Now".into_view(),
						}}
					</p>
				</div>
			</div>

			<Link
				r#type={Variant::Link}
				to={runner.get().id.to_string()}
				class="text-medium letter-sp-md text-sm mt-xs ml-auto"
			>
				"MANAGE RUNNER"
				<Icon
					icon={IconType::ChevronRight}
					color={Color::Primary}
					size={Size::ExtraSmall}
				/>
			</Link>
		</div>
	}
}
