use std::net::{IpAddr, Ipv4Addr};

use time::OffsetDateTime;

use super::EventLogCard;
use crate::prelude::*;

#[derive(Debug, Clone, Hash)]
pub struct EventLog {
	pub event: String,
	pub status: Status,
	pub user: String,
	pub ip_addr: IpAddr,
	pub date: OffsetDateTime,
}

#[component]
pub fn EventLogContainer() -> impl IntoView {
	let data = create_rw_signal(vec![WithId::new(
		Uuid::new_v4(),
		EventLog {
			event: "Something".to_string(),
			status: Status::Live,
			user: "Someone".to_string(),
			ip_addr: IpAddr::V4(Ipv4Addr::new(201, 12, 32, 12)),
			date: OffsetDateTime::now_utc(),
		},
	)]);

	view! {
		<div class="full-width fc-fs-fs mb-xl gap-md">
			<TableDashboard
				column_grids={vec![2, 2, 2, 3, 3]}
				headings={vec![
					"Event".into_view(),
					"Status".into_view(),
					"User".into_view(),
					"IP Address".into_view(),
					"Date".into_view(),
				]}
				render_rows={view! {
					<For each={move || data.get().clone()} key={move |v| v.id} let:child>
						<EventLogCard event={child.clone()} />
					</For>
				}
					.into_view()}
			/>
		</div>
	}
}
