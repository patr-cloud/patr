use super::StaticSiteItemType;
use crate::prelude::*;

#[component]
pub fn StaticSiteCard(
	/// The static site info
	#[prop(into)]
	static_site: MaybeSignal<StaticSiteItemType>,
	/// Additional Classes to add to the outer div, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"fc-fs-fs static-site-card bg-secondary-light br-sm px-xl py-md {}",
			class.get()
		)
	};
	view! {
		<div class={class}>
			<div class="fr-sb-ct full-width head pb-sm">
				<div class="fr-ct-ct">
					<h5 class="txt-white txt-md txt-thin mr-sm of-hidden txt-of-ellipsis w-20">
						{static_site.get().name}
					</h5>

					<StatusBadge status={Some(Status::Live)} />
				</div>

				<button class="fr-ct-ct">
					<Icon color={Color::Error} icon={IconType::PlayCircle} size={Size::Medium} />
				</button>
			</div>

			<a
				href="https://ca100402d79f408f98c202945cfb0310.onpatr.cloud/"
				target="_blank"
				rel="noreferrer"
				class="br-sm of-hidden mt-md full-width full-height bg-secondary-dark outline-primary-focus pos-rel site-preview-sm"
			>
				<iframe
					width="500"
					height="300"
					class="br-sm of-hidden frame pos-abs"
					src="https://ca100402d79f408f98c202945cfb0310.onpatr.cloud/"
				></iframe>
			</a>

			<div class="fr-sb-ct mt-xs full-width px-xxs">
				<Link class="letter-sp-md txt-sm fr-fs-ct">
					"MANAGE STATIC SITE"
					<Icon
						icon={IconType::ChevronRight}
						size={Size::ExtraSmall}
						color={Color::Primary}
					/>
				</Link>
			</div>
		</div>
	}
}
