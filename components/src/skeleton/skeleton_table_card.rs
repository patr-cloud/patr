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
				"w-full row-card border border-border-color py-sm px-xl br-bottom-sm 
                bg-secondary-light flex items-center justify-center {classname}"
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
							"w-full {} flex-col-{}",
							if index == column_grids.get().len() - 1 {
								"flex justify-around items-center"
							} else {
								"flex items-center justify-center px-md"
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
