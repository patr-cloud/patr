use crate::{pages::*, prelude::*};

#[component]
pub fn ImageHistoryCard(
	/// Additional Classes to add to the outer div, if any.:w
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Whether the card is active or not
	#[prop(into, optional, default = false.into())]
	active: MaybeSignal<bool>,
) -> impl IntoView {
	let class = move || {
		class.with(|cname| format!(
        "full-width px-xl py-md bg-secondary-light br-sm fc-fs-fs pos-rel deploy-summary-card txt-white {}",
        cname
    ))
	};

	view! {
		<div
			class=class
		>
			<div class="line pos-abs" />
			<div class="fr-fs-ct full-width">
				<Icon
					icon=IconType::UploadCloud
					color= if active.get() {
							Color::Success
						} else {
							Color::Info
						}
				/>

				<span class="of-hidden txt-of-ellipsis w-45 ml-sm txt-sm">
					"sha256:8414cd06d35d01807f64b4b7c99d32563a7906c54fbf11736402fd2bb57d908c"
				</span>

				<button class="btn-icon">
					<Icon icon=IconType::Copy size=Size::ExtraSmall />
				</button>

				{
					move || active.get().then(|| view! {
						<StatusBadge status=Status::Live class="ml-xxs" />
					})
				}
				<span class="txt-grey ml-auto">
					"4 Months Ago"
				</span>
			</div>

			<div class="fr-sb-ct full-width mt-sm pl-xl">
				<div class="fr-fs-ct row-card pl-sm">
					<ImageTag tag="Latest".to_owned() />
				</div>

				{
					move || (!active.get()).then(|| view! {
						<Link
							class="txt-sm letter-sp-md"
						>
							"Revert of this version"
						</Link>
					})
				}
			</div>
		</div>
	}
}
