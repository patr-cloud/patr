use models::api::workspace::database::*;
use web_sys::MouseEvent;

use crate::{pages::*, prelude::*};

#[component]
fn CopyButton(// #[prop(into)]
	// value: MaybeSignal<String>
) -> impl IntoView {
	let show_button = create_rw_signal(false);
	let copy_data = create_rw_signal(false);

	let _on_copy = move |_: MouseEvent| {
		copy_data.set(true);
	};

	create_effect(move |_| {
		show_button.set(true);
	});

	view! {
		<Show when={move || show_button.get()}>
			<Show
				when={move || !copy_data.get()}
				fallback={move || {
					view! { <Icon icon={IconType::Check} size={Size::ExtraSmall} /> }.into_view()
				}}
			>

				<button aria_label="Copy " class="btn-icon">
					<Icon icon={IconType::Copy} size={Size::ExtraSmall} />
				</button>

			</Show>
		</Show>
	}
}

#[component]
pub fn ManageDatabaseDetailsTab(
	/// The Database Item
	#[prop(into)]
	database_info: MaybeSignal<WithId<Database>>,
) -> impl IntoView {
	let store_datbase = store_value(database_info.clone());
	view! {
		<div class="full-width px-md fc-fs-fs fit-wide-screen mx-auto my-xl txt-white">

			<div class="flex mb-lg full-width">
				<label class="flex-col-2 fr-fs-fs">"Database Type"</label>

				<div class="grid grid-col-4 flex-col-10 pl-xs full-width">
					<DatabaseTypeCard
						version=4.
						database_type={Signal::derive(move || {
							store_datbase.clone().with_value(|db| db.get().engine.clone())
						})}
					/>
				</div>
			</div>

			<div class="flex mb-xs full-width mb-md">
				<div class="flex-col-2 fr-fs-ct">
					<label html_for="database-engine" class="txt-sm fr-fs-ct">
						"Name"
					</label>
				</div>
				<div class="flex-col-10 fc-fs-fs pl-xs">
					<div class="fr-fs-ct br-sm bg-secondary-light full-width py-sm px-xl">
						<span class="pl-sm">
							{move || store_datbase.with_value(|db| db.get().name.clone())}
						</span>
					</div>
				</div>
			</div>

			<div class="flex my-xs full-width mb-md">
				<div class="flex-col-2 fr-fs-ct">
					<label html_for="database-engine" class="txt-sm fr-fs-ct">
						"Region"
					</label>
				</div>
				<div class="flex-col-10 pl-xs">
					<div class="fr-fs-ct br-sm bg-secondary-light full-width py-sm px-xl">
						<span class="pl-sm">
							{move || store_datbase.with_value(|db| db.get().region.to_string())}
						</span>
					</div>
				</div>
			</div>

			<div class="flex my-xs full-width mb-md">
				<div class="flex-col-2 fr-fs-ct">
					<label html_for="database-engine" class="txt-sm fr-fs-ct">
						"Username"
					</label>
				</div>
				<div class="flex-col-10 pl-xs">
					<div class="fr-sb-ct br-sm bg-secondary-light full-width py-sm px-xl">
						<span class="pl-sm">
							{move || {
								store_datbase
									.with_value(|db| db.get().public_connection.username.clone())
							}}
						</span>
						<CopyButton />
					</div>
				</div>
			</div>

			<div class="flex my-xs full-width mb-md">
				<div class="flex-col-2 fr-fs-ct">
					<label html_for="database-engine" class="txt-sm fr-fs-ct">
						"Password"
					</label>
				</div>
				<div class="flex-col-10 pl-xs">
					<div class="fr-sb-ct br-sm bg-secondary-light full-width py-sm px-xl">
						<input
							class="txt-white px-sm"
							type="password"
							aria_label="Password"
							prop:value="********"
						/>
						<CopyButton />
					</div>

					<ChangePasswordButton />
				</div>
			</div>

			<div class="flex my-xs full-width mb-md">
				<div class="flex-col-2 fr-fs-ct">
					<label html_for="database-engine" class="txt-sm fr-fs-ct">
						"Host"
					</label>
				</div>
				<div class="flex-col-10 pl-xs">
					<div class="fr-sb-ct br-sm bg-secondary-light full-width py-sm px-xl">
						<span class="pl-sm">
							{move || {
								store_datbase
									.with_value(|db| db.get().public_connection.host.clone())
							}}
						</span>
						<CopyButton />
					</div>
				</div>
			</div>

			<div class="flex my-xs full-width mb-md">
				<div class="flex-col-2 fr-fs-ct">
					<label html_for="database-engine" class="txt-sm fr-fs-ct">
						"Port"
					</label>
				</div>
				<div class="flex-col-10 pl-xs">
					<div class="fr-sb-ct br-sm bg-secondary-light full-width py-sm px-xl">
						<span class="pl-sm">
							{move || store_datbase.with_value(|db| db.get().public_connection.port)}
						</span>
						<CopyButton />
					</div>
				</div>
			</div>

			<div class="flex my-xs full-width mb-md">
				<div class="flex-col-2 fr-fs-ct">
					<label html_for="database-engine" class="txt-sm fr-fs-ct">
						"Connection String"
					</label>
				</div>
				<div class="flex-col-10 pl-xs">
					<div class="fr-sb-ct br-sm bg-secondary-light full-width py-sm px-xl">
						<span class="pl-sm">
							{move || {
								store_datbase
									.with_value(|_db| {
										format!(
											"{}://{}:<DATABASE_PASSWORD>@{}:{}/staging",
											database_info.get().engine,
											database_info.get().public_connection.clone().username,
											database_info.get().public_connection.clone().host,
											database_info.get().public_connection.clone().port,
										)
									})
							}}
						</span>
						<CopyButton />
					</div>
				</div>
			</div>

			<div class="flex my-xs full-width mb-md">
				<div class="flex-col-2 fr-fs-ct">
					<label html_for="database-engine" class="txt-sm fr-fs-ct">
						"Managed Database Plan"
					</label>
				</div>
				<div class="flex-col-10 pl-xs">
					<DatabasePlanCard />
				</div>
			</div>
		</div>
	}
}
