use std::rc::Rc;

use ev::MouseEvent;
use leptos_query::QueryResult;

use crate::{
	prelude::*,
	queries::{list_deployments_query, AllDeploymentsTag},
};

/// The Deployment Dashboard Pagination Footer
#[component]
pub fn DeploymentDashboardFooter(
	/// Current Page
	#[prop(into)]
	current_page: RwSignal<usize>,
) -> impl IntoView {
	let on_click_prev = move |_: &MouseEvent| {
		if current_page.get() > 0 {
			current_page.set(current_page.get() - 1);
		} else {
			current_page.set(0);
		}
	};

	let on_click_next = move |_: &MouseEvent| {
		current_page.set(current_page.get() + 1);
	};

	view! {
		<div class="flex justify-center items-center text-white gap-xl mt-auto pb-xl">
			<Link
				on_click={Rc::new(on_click_prev)}
				style_variant={LinkStyleVariant::Contained}
				r#type={Variant::Button}
			>
				<Icon
					icon={IconType::ChevronLeft}
					size={Size::ExtraSmall}
					color={Color::Black}
				/>
				"Prev"
			</Link>
			// <Link
			// 	style_variant={LinkStyleVariant::Plain}
			// 	r#type={Variant::Link}
			// >
			// 	"1"
			// </Link>
			<Link
				style_variant={LinkStyleVariant::Plain}
				r#type={Variant::Link}
			>
				{move || current_page.get() + 1}
			</Link>
			// <Link
			// 	style_variant={LinkStyleVariant::Outlined}
			// 	r#type={Variant::Link}
			// >
			// 	"3"
			// </Link>
			<Link
				on_click={Rc::new(on_click_next)}
				style_variant={LinkStyleVariant::Contained}
				r#type={Variant::Button}
			>
				"Next"
				<Icon
					icon={IconType::ChevronRight}
					size={Size::ExtraSmall}
					color={Color::Black}
				/>
			</Link>
		</div>
	}
}
