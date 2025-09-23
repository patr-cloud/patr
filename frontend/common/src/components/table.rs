use leptos::*;

use crate::prelude::*;
/// Table component to list different things in table format
#[component]
pub fn Table(
	/// The table headers
	headers: Vec<String>,
	/// The table data rows
	data: Vec<Vec<String>>,
) -> impl IntoView {
	// prepare rows: either use provided data or synthesize placeholder rows
	let rows: Vec<Vec<String>> = data.clone();
	// use custom classes if provided
	// let wrapper_class = class.unwrap_or_else(|| "w-full h-full".to_string());

	view! {
	<div class="min-h-screen w-full p-6">
		<table class="w-full h-full min-w-full divide-y divide-gray-700 table-auto text-white">
			<thead class="bg-[#302b63] text-orange-300">
				<tr>
					{headers.iter().map(|h| view!{ <th class="px-4 py-2 text-left text-sm font-semibold uppercase tracking-wider">{h.clone()}</th> }).collect::<Vec<_>>()}
				</tr>
			</thead>


			<tbody class="divide-y divide-gray-700">
				{rows.into_iter().map(|row| view!{
					<tr>
						{row.into_iter().enumerate().map(|(ci, cell)| {
							// Apply conditional styling for Connected / Disconnected
							let cell_class = if ci == 1 { // second column (status)
								if cell.to_lowercase() == "connected" {
								"px-4 py-3 whitespace-nowrap text-sm text-green-400"
								} else if cell.to_lowercase() == "disconnected" {
								"px-4 py-3 whitespace-nowrap text-sm text-red-400"
								} else {
								"px-4 py-3 whitespace-nowrap text-sm"
								}
								} else {
								"px-4 py-3 whitespace-nowrap text-sm"
							};


							view! { <td class={cell_class}>{cell}</td> }
						 }).collect::<Vec<_>>()}
					</tr>
				}).collect::<Vec<_>>()}
			</tbody>
		</table>
	</div>
	}
}
