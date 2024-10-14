use crate::prelude::*;

#[component]
pub fn CreateDatabaseHeader() -> impl IntoView {
	view! {
		<ContainerHead>
			<PageTitleContainer
				page_title_items={vec![
					PageTitleItem {
						title: "Infrastructure".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::End,
						variant: PageTitleVariant::Heading,
					},
					PageTitleItem {
						title: "Database".to_owned(),
						link: Some("/database".to_owned()),
						icon_position: PageTitleIconPosition::End,
						variant: PageTitleVariant::SubHeading,
					},
					PageTitleItem {
						title: "Create Database".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::None,
						variant: PageTitleVariant::Text,
					},
				]}
				description_title={
					Some("Create a new Database here.".to_owned())
				}
				description_link={
					Some("https://docs.patr.cloud/features/databases/".to_owned())
				}
			/>
		</ContainerHead>
	}
}
