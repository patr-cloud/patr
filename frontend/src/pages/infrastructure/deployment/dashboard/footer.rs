use std::rc::Rc;

use ev::MouseEvent;

use crate::prelude::*;

/// Get The Number of Pages from the Total Resource Count
fn get_num_pages(total_count: usize) -> usize {
	let num_pages = total_count / constants::RESOURCES_PER_PAGE;
	if total_count % constants::RESOURCES_PER_PAGE != 0 {
		num_pages + 1
	} else {
		num_pages
	}
}

/// The Deployment Dashboard Pagination Footer
#[component]
pub fn DeploymentDashboardFooter(
	/// Current Page
	#[prop(into)]
	current_page: RwSignal<usize>,
	#[prop(into)] total_count: MaybeSignal<usize>,
) -> impl IntoView {
	let total_pages = Signal::derive(move || get_num_pages(total_count.get()));

	let on_click_prev = move |_: &MouseEvent| {
		if current_page.get() > 0 {
			current_page.set(current_page.get() - 1);
		} else {
			current_page.set(0);
		}
	};

	let on_click_next = move |_: &MouseEvent| {
		if current_page.get() < total_pages.get() - 1 {
			current_page.set(current_page.get() + 1);
		} else {
			current_page.set(total_count.get() - 1)
		}
	};

	let on_click_page = move |page: usize| {
		if page <= 0 {
			current_page.set(0);
		}

		if page >= total_pages.get() {
			current_page.set(total_pages.get());
		}

		current_page.set(page - 1);
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

			<For
				each={move || (1..=total_pages.get()).collect::<Vec<_>>()}
				key={|state| state.clone()}
				let:page
			>
				<Link
					 on_click={Rc::new(move |_| on_click_page(page))}
					style_variant={LinkStyleVariant::Plain}
					r#type={Variant::Button}
				>
					{move || page}
				</Link>
			</For>

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
