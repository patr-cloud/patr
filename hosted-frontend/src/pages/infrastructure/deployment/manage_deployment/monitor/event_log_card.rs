use super::EventLog;
use crate::prelude::*;

#[component]
pub fn EventLogCard(
	/// The Event Log Item
	#[prop(into)]
	event: MaybeSignal<WithId<EventLog>>,
) -> impl IntoView {
	view! {
		<tr class="fr-ct-ct full-width bd-light br-bottom-sm bg-secondary-light px-xl row-card">
			<td class="flex-col-2 fr-ct-ct">{event.get().data.event.clone()}</td>
			<td class="flex-col-2 fr-ct-ct">{event.get().data.status.get_status_text()}</td>
			<td class="flex-col-2 fr-ct-ct">{event.get().data.user.clone()}</td>
			<td class="flex-col-3 fr-ct-ct">{event.get().data.ip_addr.to_string()}</td>
			<td class="flex-col-3 fr-ct-ct">{event.get().data.date.to_string()}</td>
		</tr>
	}
}
