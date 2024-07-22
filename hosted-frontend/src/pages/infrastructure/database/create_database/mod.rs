mod head;

use std::{rc::Rc, str::FromStr};

use models::api::workspace::database::DatabaseEngine;
use strum::VariantNames;
use utils::FromToStringCodec;

pub use self::head::*;
use crate::{pages::DatabaseTypeCard, prelude::*};

#[derive(Clone, Debug)]
pub struct DatabaseInfo {
	name: Option<String>,
	database_type: Option<DatabaseEngine>,
}

#[component]
pub fn CreateDatabase() -> impl IntoView {
	let (access_token, _) = use_cookie::<String, FromToStringCodec>(constants::ACCESS_TOKEN);
	let (current_workspace_id, _) =
		use_cookie::<String, FromToStringCodec>(constants::LAST_USED_WORKSPACE_ID);

	let selected_runner = create_rw_signal("".to_string());
	let database_info = create_rw_signal(DatabaseInfo {
		name: None,
		database_type: None,
	});

	let runner_list = create_resource(
		move || (access_token.get(), current_workspace_id.get()),
		move |(access_token, workspace_id)| async move {
			list_runners(workspace_id, access_token).await
		},
	);

	let create_db_fn = create_server_action::<CreateDatabaseFn>();

	let machine_type = Uuid::new_v4().to_string();
	let runner_id = Uuid::new_v4().to_string();

	view! {
		<CreateDatabaseHeader />
		<ContainerBody class="px-xl">
			 <ActionForm action={create_db_fn} class="full-width fc-sb-ct full-height">
				<input type="hidden" name="access_token" value={access_token.get()} />
				<input type="hidden" name="workspace_id" value={current_workspace_id.get()} />
				<input type="hidden" name="num_nodes" value={2} />
				<input type="hidden" name="version" value={"13."} />
				<input type="hidden" name="machine_type" value={machine_type} />
				<input type="hidden" name="runner_id" value={runner_id} />

				<div class="full-width px-md fc-fs-fs fit-wide-screen mx-auto my-xl txt-white">
					<div class="flex mb-lg full-width">
						<label class="flex-col-2 fr-fs-fs">"Database Type"</label>

						<div class="grid-col-4 flex-col-10 pl-xs full-width gap-xl">
							{
								DatabaseEngine::VARIANTS
									.into_iter()
									.map(|engine| match DatabaseEngine::from_str(engine.to_owned()) {
										Ok(engine) => view! {
											<DatabaseTypeCard
												version=4.
												database_type={engine.clone()}
												is_selected={false}
											/>
										}.into_view(),
										Err(_) =>  view! {
											<p class="txt-medium txt-white">"Cannot Load Runners"</p>
										}.into_view()
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
								name="name"
								id="name"
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
								<ul class="fr-fs-fs gap-sm full-width f-wrap">
									{
										move || match runner_list.get() {
											Some(Ok(runners)) => view! {
												<For
													each={move || runners.runners.clone()}
													key={|state| state.id.clone()}
													let:child
												>
													 <li class="flex-col-3">
														<label class="fr-fs-ct gap-md bg-secondary-light fr-fs-ct full-width txt-white br-sm py-sm px-xl">
															<input
																type="radio"
																value={child.id.to_string()}
																name="runner"
															/>
															<p>{child.name.clone()}</p>
														</label>
													</li>
												</For>
											}.into_view(),
											_ => view! {
												<li class="px-xl py-sm ul-light fr-fs-ct full-width br-bottom-sm txt-white">
													"Error Loading Runners"
												</li>
											}.into_view()
										}
									}
								</ul>
							</Transition>
						</div>
					</div>
				</div>

				<div class="fr-fe-ct gap-md full-width fit-wide-screen pt-md pb-xl px-md">
					<Link
						r#type={Variant::Link}
						style_variant={LinkStyleVariant::Plain}
						to="/database"
					>
						"BACK"
					</Link>
					<Link
						r#type={Variant::Button}
						style_variant={LinkStyleVariant::Contained}
						should_submit=true
					>
						"NEXT"
					</Link>
				</div>
			</ActionForm>
		</ContainerBody>
	}
}
