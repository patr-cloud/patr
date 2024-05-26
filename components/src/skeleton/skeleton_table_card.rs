use crate::imports::*;

#[component]
pub fn SkeletonTableCard(
	/// Additional class names to apply to the outer tr, if any
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// Flex Grid Ratio of columns
	#[prop(into, optional)]
	column_grids: MaybeSignal<Vec<i32>>,
) -> impl IntoView {
	let class = move || {
		class.with(|classname| {
			format!(
				"full-width row-card bd-light py-sm px-xl br-bottom-sm 
                bg-secondary-light fr-ct-ct {classname}"
			)
		})
	};

	view! {
		<tr class={class}>

			{column_grids
				.get()
				.into_iter()
				.enumerate()
				.map(|(index, columns)| {
					view! {
						<td class={format!(
							"full-width {} flex-col-{}",
							if index == column_grids.get().len() - 1 {
								"fr-sa-ct"
							} else {
								"fr-ct-ct px-md"
							},
							columns,
						)}>
							<Skeleton enable_full_width=true enable_full_height=true/>
						</td>
					}
				})
				.collect_view()}

		</tr>
	}
}
