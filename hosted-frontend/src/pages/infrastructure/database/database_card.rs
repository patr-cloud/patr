use std::vec;

use super::DatabaseItem;
use crate::prelude::*;

pub struct CardItem {
	label: &'static str,
	value: String,
}

#[component]
pub fn DatabaseCard(
	/// The Database Info
	#[prop(into)]
	deployment: MaybeSignal<DatabaseItem>,
	/// Additional Classes to add to the outer div, if any.:w
	#[prop(into, optional)]
	class: MaybeSignal<String>,
) -> impl IntoView {
	let class = move || {
		format!(
			"bg-secondary-light br-sm p-lg fc-fs-fs database-card {}",
			class.get()
		)
	};
	let items = vec![
		CardItem {
			label: "REGION",
			value: deployment.get().region,
		},
		CardItem {
			label: "ENGINE",
			value: deployment.get().engine,
		},
		CardItem {
			label: "VERSION",
			value: deployment.get().version,
		},
		CardItem {
			label: "PLAN",
			value: deployment.get().plan,
		},
	];

	view! {
		<div class=class>
			<div class="fr-fs-ct full-width px-xxs">
				<h4 class="txt-md txt-primary of-hidden txt-of-ellipsis w-25">
					{deployment.get().name}
				</h4>

				<StatusBadge
					status=Status::Live
					class="mb-xxs ml-xxs"
				/>
			</div>

			<div class="fr-fs-fs txt-white full-width f-wrap my-auto">
				{
					items
						.into_iter()
						.map(|item| view! {
							<div class="half-width p-xxs">
								<div class="bg-secondary-medium br-sm px-lg py-sm fc-ct-fs">
									<span className="letter-sp-md txt-xxs txt-grey">
										{item.label}
									</span>
									<span className="txt-primary w-15 txt-of-ellipsis of-hidden">
										{item.value}
									</span>
								</div>
							</div>
						})
						.collect::<Vec<_>>()
				}
			</div>
			<div class="fr-fs-ct mt-xs full-width px-xxs">
				<Link class="letter-sp-md  txt-sm fr-fs-ct">
					"MANAGE DATABASE"
					<Icon
						icon=IconType::ChevronRight
						size=Size::ExtraSmall
						color=Color::Primary
					/>
				</Link>
			</div>
		</div>
	}
}
