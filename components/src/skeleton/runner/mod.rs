use std::rc::Rc;

use leptos::component;

use crate::imports::*;

/// The skeleton for the runner component in the runner list.
#[component]
pub fn RunnerCardSkeleton() -> impl IntoView {
	view! {
		<div
			class="bg-secondary-light flex flex-col items-start justify-start px-lg py-md br-sm text-white gap-xs"
		>
			<div class="w-full h-6 flex items-center justify-start gap-md">
				<Skeleton class="w-1/2 h-full" />
			</div>

			<div class="flex-2 w-full gap-xs flex items-center justify-center h-full grow">
				<Skeleton class="bg-secondary-medium br-sm px-lg py-sm flex flex-col items-start justify-center w-full" />
			</div>

			<Link
				r#type={Variant::Button}
				disabled={true}
				on_click={Rc::new(move |_| ())}
				class="text-medium letter-sp-md text-sm mt-xs ml-auto"
			>
				"MANAGE RUNNER"
				<Icon
					icon=IconType::ChevronRight
					color=Color::Primary
					size=Size::ExtraSmall
				/>
			</Link>
		</div>
	}
}
