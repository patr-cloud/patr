use crate::prelude::*;

#[component]
pub fn ManageDatabases() -> impl IntoView {
	view! {
		<ContainerMain class="full-width full-height mb-md">
			<ContainerHead>
				<div class="fr-sb-ct full-width">
					<div class="fc-fs-fs">
						<PageTitleContainer>
							<PageTitle icon_position=PageTitleIconPosition::End>
								"Infrastructure"
							</PageTitle>
							<PageTitle
								icon_position=PageTitleIconPosition::End
								variant=PageTitleVariant::SubHeading
							>
								"Database"
							</PageTitle>
							<PageTitle variant=PageTitleVariant::Text>
								"Database Name"
							</PageTitle>
						</PageTitleContainer>
					</div>

					<Link
						r#type=Variant::Button
						style_variant=LinkStyleVariant::Contained
						color=Color::Error
						class="fr-ct-ct gap-xs"
					>
						<Icon
							icon=IconType::Trash2
							size=Size::ExtraSmall
							color=Color::White
						/>
						"DELETE"
					</Link>
				</div>

				<Tabs
					tab_items=vec![
						TabItem {
							name: "Details".to_owned(),
							path: "/staging".to_owned()
						},
					]
				/>
			</ContainerHead>

			<ContainerBody class="px-xxl py-xl gap-md">
				<ManageDatabaseDetailsTab />
			</ContainerBody>
		</ContainerMain>
	}
}

mod details_tab;

pub use self::details_tab::*;
