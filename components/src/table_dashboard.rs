use crate::imports::*;

#[component]
pub fn TableDashboard(
	/// Headings of the Table
	#[prop(into)]
	headings: Vec<View>,
	/// Additional class names to apply to the outer class, if any.
	#[prop(into, optional)]
	class: MaybeSignal<String>,
	// #[prop(into, optional)] children: Children,
) -> impl IntoView {
	let class = move || {
		format!(
			"fc-fs-fs br-sm of-hidden full-width txt-white {}",
			class.get()
		)
	};

	view! {
		<table class=class>
			<thead class="fr-ct-ct py-sm bg-secondary-medium full-width">
				<tr class="fr-ct-ct px-xl full-width">
						{
							headings.into_iter()
								.map(|x| view! {
									<th class="fr-ct-ct txt-sm txt-medium">
										{x}
									</th>
								})
								.collect_view()
						}
				</tr>
			</thead>
			// {children()}

			<tbody class="full-width full-height fc-fs-fs">
				<tr class="bg-secondary-light full-width fr-ct-ct px-xl bd-light br-smooth-sm row-card">
					<td class="flex-col-11 fr-fs-ct">"Email Password"</td>
					<td class="flex-col-1 fr-sa-ct">
						<button>
							<Icon icon=IconType::Edit size=Size::ExtraSmall />
						</button>
						<button>
							<Icon icon=IconType::Trash2 size=Size::ExtraSmall color=Color::Error />
						</button>
					</td>
				</tr>
			</tbody>
		</table>
	}
}
