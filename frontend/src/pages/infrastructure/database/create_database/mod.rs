mod head;

use std::{rc::Rc, str::FromStr};

use models::api::workspace::database::DatabaseEngine;
use strum::VariantNames;

pub use self::head::*;
use crate::{pages::DatabaseTypeCard, prelude::*};

#[derive(Clone, Debug)]
pub struct DatabaseInfo {
	name: Option<String>,
	database_type: Option<DatabaseEngine>,
}

#[component]
pub fn CreateDatabase() -> impl IntoView {
	let (state, _) = AuthState::load();
	let access_token = Signal::derive(move || state.get().get_access_token());
	let current_workspace_id = Signal::derive(move || state.get().get_last_used_workspace_id());

	let selected_runner = create_rw_signal("".to_string());
	let database_info = create_rw_signal(DatabaseInfo {
		name: None,
		database_type: None,
	});

	let runner_list = create_resource(
		move || (access_token.get(), current_workspace_id.get()),
		move |(access_token, workspace_id)| async move {
			list_runners(access_token, workspace_id.unwrap()).await
		},
	);

	let name_error = create_rw_signal("".to_string());
	let db_type_error = create_rw_signal("".to_string());
	let runner_error = create_rw_signal("".to_string());

	let on_submit = move || {
		logging::log!("{:?}\n{}", database_info.get(), selected_runner.get());
		database_info.with(|info| {
			name_error.set("".to_string());
			db_type_error.set("".to_string());
			runner_error.set("".to_string());

			if info.name.is_none() {
				name_error.set("Please bestow thy database with a moniker🖋️".to_string());
				return;
			}
			if info.database_type.is_none() {
				db_type_error.set("Please Provide a Database Engine!".to_string());
			}
		});
		if selected_runner.get().clone().is_empty() {
			runner_error.set("Please select a Runner".to_string());
			return;
		}

		spawn_local(async move {
			_ = create_database(
				// I'm checking that the value is not none above, hence
				// it's safe to unwrap
				database_info.get().name.unwrap(),
				1,
				// I'm checking that the value is not none above, hence
				// it's safe to unwrap
				database_info.get().database_type.unwrap(),
				access_token.get(),
				current_workspace_id.get(),
				Uuid::new_v4().to_string(),
				"4.".to_string(),
				selected_runner.get(),
			)
			.await;
		})
	};

	view! {
		<CreateDatabaseHeader />
		<ContainerBody class="px-xl">
			<div class="full-width px-md fc-fs-fs fit-wide-screen mx-auto my-xl txt-white">
				<div class="flex mb-lg full-width">
					<label class="flex-col-2 fr-fs-fs">"Database Type"</label>

					<div class="grid-col-4 flex-col-10 pl-xs full-width gap-sm">
						// <For
						// each=
						// key=|state| state.clone()
						// let:engine
						// >
						// </For>
						{DatabaseEngine::VARIANTS
							.iter()
							.map(|engine| match DatabaseEngine::from_str(engine.to_owned()) {
								Ok(engine) => {
									view! {
										<DatabaseTypeCard
											version=4.
											database_type={engine.clone()}
											is_selected={Signal::derive(move || false)}
											on_click={move |engine| {
												database_info
													.update(|info| {
														if info.database_type.is_some() {
															info.database_type = None
														} else {
															info.database_type = Some(engine)
														}
													})
											}}
										/>
									}
										.into_view()
								}
								Err(_) => view! {}.into_view(),
							})
							.collect_view()}
						<Show when={move || !db_type_error.get().clone().is_empty()}>
							<Alert r#type={AlertType::Error} class="mt-xs">
								{move || db_type_error.get().clone()}
							</Alert>
						</Show>
					</div>
				</div>

				<div class="flex full-width mb-md">
					<div class="flex-col-2 fr-fs-ct">
						<label html_for="database-engine" class="txt-sm fr-fs-ct">
							"Name"
						</label>
					</div>

					<div class="flex-col-10 fc-fs-fs pl-xs">
						<Input
							r#type={InputType::Text}
							class="full-width"
							placeholder="Database Name"
							value={Signal::derive(move || {
								database_info.get().name.unwrap_or_default()
							})}
							on_input={Box::new(move |ev| {
								ev.prevent_default();
								database_info
									.update(|info| info.name = Some(event_target_value(&ev)))
							})}
						/>

						<Show when={move || !name_error.get().clone().is_empty()}>
							<Alert r#type={AlertType::Error} class="mt-xs">
								{move || name_error.get().clone()}
							</Alert>
						</Show>
					</div>
				</div>

				<div class="flex my-xs full-width mb-md">
					<div class="flex-col-2 fr-fs-ct">
						<label html_for="database-engine" class="txt-sm fr-fs-ct">
							"Runner"
						</label>
					</div>
					<div class="flex-col-10 pl-xs">
						<Transition>
							<InputDropdown
								placeholder="Choose A Runner"
								class="full-width"
								value={selected_runner}
								options={Signal::derive(move || match runner_list.get() {
									Some(Ok(data)) => {
										data.runners
											.iter()
											.map(|runner| InputDropdownOption {
												id: runner.id.to_string().clone(),
												disabled: runner.data.connected,
												label: runner.name.clone(),
											})
											.collect::<Vec<_>>()
									}
									_ => {
										vec![
											InputDropdownOption {
												id: "error".to_string(),
												disabled: true,
												label: "Error Loading".to_string(),
											},
										]
									}
								})}
							/>
						</Transition>

						<Show when={move || !runner_error.get().clone().is_empty()}>
							<Alert r#type={AlertType::Error} class="mt-xs">
								{move || runner_error.get().clone()}
							</Alert>
						</Show>
					</div>
				</div>
			</div>

			<div class="fr-fe-ct gap-md full-width fit-wide-screen mx-auto mt-auto pt-md pb-xl px-md">
				<Link r#type={Variant::Link} style_variant={LinkStyleVariant::Plain} to="/database">
					"BACK"
				</Link>
				<Link
					r#type={Variant::Button}
					style_variant={LinkStyleVariant::Contained}
					should_submit=true
					on_click={Rc::new(move |ev| {
						ev.prevent_default();
						on_submit();
					})}
				>
					"NEXT"
				</Link>
			</div>
		</ContainerBody>
	}
}
