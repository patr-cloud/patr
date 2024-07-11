use models::api::workspace::runner::Runner;

use crate::prelude::*;

#[component]
pub fn RunnerCard(
	/// The Runner Info
	#[prop(into)]
	runner: MaybeSignal<WithId<Runner>>,
) -> impl IntoView {
	view! {
		<div
			class="bg-secondary-light fc-fs-fs px-lg py-md br-sm txt-white gap-xs"
		>
			<p class="txt-md txt-primary w-25 txt-of-ellipsis of-hidden">
				{runner.get().name.clone()}
			</p>

			<div class="flex-2 full-width gap-xs fr-ct-ct">
				<div class="bg-secondary-medium br-sm px-lg py-sm fc-ct-fs full-width">
					<small class="letter-sp-md txt-xxs txt-grey">
						"LAST SEEN"
					</small>
					<p class="txt-primary w-15 txt-of-ellipsis of-hidden">
						{match runner.get().last_seen.clone() {
							Some(date) => date.to_string().into_view(),
							None => "Just Now".into_view()
						}}
					</p>
				</div>
			</div>

			<Link
				r#type={Variant::Link}
				to="some_route"
				class="txt-medium letter-sp-md txt-sm mt-xs ml-auto"
			>
				"MANAGE RUNNER"
				<Icon
					icon=IconType::ChevronRight
					color=Color::Primary
					size=Size::ExtraSmall
				/>
			</Link>
		</div>
	}
}
