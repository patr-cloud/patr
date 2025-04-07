use std::vec;

use models::api::workspace::database::Database;

use crate::prelude::*;

/// Database Card Item
pub struct CardItem {
	/// The label of the card Item
	label: &'static str,
	/// The Value of the item
	value: String,
}

#[component]
pub fn DatabaseCard(
	/// The Database Info
	#[prop(into)]
	database: MaybeSignal<WithId<Database>>,
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
			value: database.get().region.to_string(),
		},
		CardItem {
			label: "ENGINE",
			value: database.get().engine.to_string(),
		},
		CardItem {
			label: "VERSION",
			value: database.get().version.clone(),
		},
		CardItem {
			label: "PLAN",
			value: database.get().database_plan_id.to_string(),
		},
	];

	view! {
		<div class={class}>
			<div class="fr-fs-ct full-width px-xxs">
				<h4 class="txt-md txt-primary of-hidden txt-of-ellipsis w-25">
					{database.get().name.clone()}
				</h4>

				<StatusBadge
					class="mb-xxs ml-xxs"
					status={
						let database = database.clone();
						Signal::derive(move || Some(
							Status::from_database_status(database.get().status.clone()),
						))
					}
				/>
			</div>

			<div class="fr-fs-fs txt-white full-width f-wrap my-auto">
				{items
					.into_iter()
					.map(|item| {
						view! {
							<div class="half-width p-xxs">
								<div class="bg-secondary-medium br-sm px-lg py-sm fc-ct-fs">
									<span class="letter-sp-md txt-xxs txt-grey">{item.label}</span>
									<span class="txt-primary w-15 txt-of-ellipsis of-hidden">
										{item.value}
									</span>
								</div>
							</div>
						}
					})
					.collect::<Vec<_>>()}

			</div>
			<div class="fr-fs-ct mt-xs full-width px-xxs">
				<Link
					r#type={Variant::Link}
					to={database.get().id.to_string()}
					class="letter-sp-md txt-sm fr-fs-ct"
				>
					"MANAGE DATABASE"
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
