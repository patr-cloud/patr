use models::api::workspace::deployment::deploy_history::DeploymentDeployHistory;

use crate::{pages::*, prelude::*};

#[component]
pub fn ImageHistoryCard(
	/// Additional Classes to add to the outer div, if any.:w
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Whether the card is active or not
	#[prop(into, optional, default = false.into())]
	active: MaybeSignal<bool>,
	/// The Deployment Info
	#[prop(into)]
	deploy_history: MaybeSignal<DeploymentDeployHistory>,
) -> impl IntoView {
	let class = move || {
		class.with(|cname| format!(
			"w-full px-xl py-md bg-secondary-light rounded-sm flex flex-col items-start justify-start pos-rel deploy-summary-card text-white {}",
			cname
    	))
	};

	view! {
		<div class={class}>
			<div class="line absolute"></div>
			<div class="flex justify-start items-center w-full">
				<Icon
					icon={IconType::UploadCloud}
					color={if active.get() { Color::Success } else { Color::Info }}
				/>

				<span class="overflow-hidden text-ellipsis w-[45ch] ml-sm text-sm">
					{deploy_history.get().clone().image_digest}
				</span>

				<button class="btn-icon">
					<Icon icon={IconType::Copy} size={Size::ExtraSmall} />
				</button>

				{move || {
					active
						.get()
						.then(|| {
							view! { <StatusBadge status={Some(Status::Live)} class="ml-xxs" /> }
						})
				}}

				<span class="text-grey ml-auto">
					{deploy_history.get().clone().created.to_string()}
				</span>
			</div>

			<div class="flex justify-between items-center w-full mt-sm pl-xl">
				<div class="flex justify-start items-center row-card pl-sm">
					<ImageTag tag={"Latest".to_owned()} />
				</div>

				{move || {
					(!active.get())
						.then(|| {
							view! {
								<Link class="text-sm tracking-[1px]">"Revert of this version"</Link>
							}
						})
				}}

			</div>
		</div>
	}
}
