use std::collections::BTreeMap;

use crate::{pages::*, prelude::*};

#[component]
pub fn ManageDeploymentDetailsTab() -> impl IntoView {
	view! {
		<div class="fc-fs-fs full-width px-xl pb-xl mt-xl txt-white gap-md fit-wide-screen mx-auto">
			<div class="flex full-width">
				<div class="flex-col-2 fr-fs-fs">
					<label class="txt-white txt-sm mt-sm fr-fs-ct" html_for="name">
						"Name"
					</label>
				</div>

				<div class="flex-col-10 fc-fs-fs">
					<Input class="full-width"/>
				</div>
			</div>

			<div class="flex full-width mb-md">
				<div class="flex-col-2 fr-fs-fs">
					<label class="txt-white txt-sm mt-sm fr-fs-ct" html_for="registry">
						"Registry"
					</label>
				</div>

				<div class="flex-col-10">
					<Textbox disabled=true value={"docker.io".into_view()}/>
				</div>
			</div>

			<div class="flex full-width">
				<div class="flex-col-2 fr-fs-fs">
					<label html_for="image-details">"Image Details"</label>
				</div>

				<div class="flex-col-7">
					<Textbox disabled=true value={"nginx".into_view()}/>
				</div>
				<div class="flex-col-3 pl-md">
					<Textbox disabled=true value={"latest".into_view()}/>
				</div>
			</div>

			<div class="flex full-width">
				<div class="flex-col-2 fr-fs-fs">
					<label html_for="image-details">"Region"</label>
				</div>
				<div class="flex-col-10 fc-fs-fs">
					<Textbox value={"Singapore".into_view()} disabled=true/>
				</div>
			</div>

			<PortInput
				is_update_screen=true
			/>

			<EnvInput envs_list={BTreeMap::new()}/>

			<ConfigMountInput mount_points={vec!["/x/y/path".to_owned()]}/>

			<ProbeInput probe_type={ProbeInputType::Startup}/>

			<ProbeInput probe_type={ProbeInputType::Liveness}/>
		</div>
	}
}
