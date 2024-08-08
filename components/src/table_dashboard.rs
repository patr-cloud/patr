use crate::imports::*;

#[component]
pub fn TableDashboard(
	/// Flex Grid Ratio of columns
	#[prop(into, optional)]
	column_grids: Vec<i32>,
	/// Headings of the Table
	#[prop(into)]
	headings: Vec<View>,
	/// Additional class names to apply to the outer table, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	/// All the rows to be rendered, does not iterate,
	/// send the <For /> component or all the rows in the component.
	render_rows: View,
) -> impl IntoView {
	let class = move || {
		format!(
			"fc-fs-fs br-sm of-hidden full-width txt-white {}",
			class.get()
		)
	};

	view! {
		<table class={class}>
			<thead class="flex items-center justify-center py-sm
				bg-secondary-medium full-width rounded-tl-sm rounded-tr-sm"
			>
				<tr class="flex items-center justify-center px-xl w-full">

					{headings
						.into_iter()
						.enumerate()
						.map(|(i, heading)| {
							view! {
								<th class={format!(
									"flex items-center justify-center text-sm text-medium flex-col-{}",
									column_grids[i],
								)}>{heading}</th>
							}
						})
						.collect_view()}

				</tr>
			</thead>

			<tbody class="w-full h-full flex flex-col justify-start items-start">
				{render_rows}
			</tbody>
		</table>
	}
}
