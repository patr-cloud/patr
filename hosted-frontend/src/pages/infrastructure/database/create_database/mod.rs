mod head;

use std::str::FromStr;

use models::api::workspace::database::DatabaseEngine;
use strum::VariantNames;
use utils::FromToStringCodec;

pub use self::head::*;
use crate::{pages::DatabaseTypeCard, prelude::*};

#[component]
pub fn CreateDatabase() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let runner_list = create_resource(
		move || (access_token.get(), current_workspace_id.get()),
		move |(access_token, workspace_id)| async move {
			list_runners(workspace_id, access_token).await
		},
	);

	view! {
		<CreateDatabaseHeader />
		<ContainerBody class="px-xl">
			<div class="full-width px-md fc-fs-fs fit-wide-screen mx-auto my-xl txt-white">
				<div class="flex mb-lg full-width">
					<label class="flex-col-2 fr-fs-fs">"Database Type"</label>

					<div class="grid-col-4 flex-col-10 pl-xs full-width gap-sm">
						{
							DatabaseEngine::VARIANTS
								.into_iter()
								.map(|engine| match DatabaseEngine::from_str(engine.to_owned()) {
									Ok(engine) => view! {
										<DatabaseTypeCard
											version=4.
											database_type={engine}
										/>
									}.into_view(),
									Err(_) =>  view! {}.into_view()
								})
								.collect_view()
						}
					</div>
				</div>

				<div class="flex mb-xs full-width mb-md">
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
						/>
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
								options={Signal::derive(
									move || match runner_list.get() {
										Some(Ok(data)) => {
											data.runners
												.iter()
												.map(|runner| InputDropdownOption {
													id: runner.id.to_string().clone(),
													disabled: runner.data.connected,
													label: runner.name.clone()
												})
												.collect::<Vec<_>>()
										},
										_ => vec![
											InputDropdownOption {
												id: "error".to_string(),
												disabled: true,
												label: "Error Loading".to_string()
											}
										]
									})
								}
							/>
						</Transition>
					</div>
				</div>
			</div>
		</ContainerBody>
	}
}
