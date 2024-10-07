use std::rc::Rc;

use ev::MouseEvent;

use crate::prelude::*;

#[component]
pub fn UrlDashboardHead(
	/// On Click Create Managed Url
	#[prop(into, optional, default = Callback::new(|_| ()))]
	on_click_create: Callback<MouseEvent>,
) -> impl IntoView {
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
						title: "Managed URL".to_owned(),
						link: None,
						icon_position: PageTitleIconPosition::None,
						variant: PageTitleVariant::SubHeading,
					},
				]}
				description_title={
					Some("Create and Manage URLs with ease using Patr".to_owned())
				}
				description_link={
					Some("https://docs.patr.cloud/features/managed-urls/".to_owned())
				}
				action_buttons={Some(view! {
					<Link
						r#type={Variant::Button}
						style_variant={LinkStyleVariant::Contained}
						on_click={Rc::new(move |ev| { on_click_create.call(ev.clone()) })}
					>
						"CREATE MANAGED URL"
						<Icon
							icon={IconType::Plus}
							size={Size::ExtraSmall}
							class="ml-xs"
							color={Color::Black}
						/>
					</Link>
				}.into_view())}
			/>
		</ContainerHead>
	}
}
