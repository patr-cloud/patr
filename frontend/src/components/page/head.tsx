import { For, JSX, mergeProps, Show } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";
import Link from "~/components/link";

export interface HeadTabProps {
	/** Additional CSS classes for the tab container */
	class?: MaybeAccessor<string>;
	/** Additional CSS classes for the tab buttons */
	buttonClass?: MaybeAccessor<string>;
	/** Tab */
	tab: MaybeAccessor<string>;
	/** Callback when tab changes */
	onTabChange?: (value: string) => void;
	/** Tab Items */
	tabItems: Array<{
		/** Button Class */
		class?: MaybeAccessor<string>;
		/** The label of the tab */
		label: string;
		/** The value of the tab */
		value: string;
		/** OnClick handler for the tab */
		onClick?: (value: string) => void;
	}>;
}

const HeadTab = (rawProps: HeadTabProps) => {
	const props = mergeProps(
		{
			class: "",
			buttonClass: "",
		},
		rawProps
	);

	return (
		<div class={`w-full text-white flex gap-4 overflow-x-auto ${get(props.class)}`}>
			<For each={props.tabItems}>
				{(item) => (
					<button
						onClick={() => {
							if (item.onClick) {
								item.onClick(item.value);
							} else if (props.onTabChange) {
								props.onTabChange(item.value);
							}
						}}
						class={`pb-2 px-2 border-b-2 ${
							get(props.tab) === item.value ? "border-primary" : "border-none"
						} ${get(props.buttonClass)} ${get(item.class) || ""}`}
					>
						{item.label}
					</button>
				)}
			</For>
		</div>
	);
};

type Breadcrumb = {
	label: string;
	url?: string;
};

interface PageContainerHeadProps {
	/** Breadcrumbs to be displayed at the top of the header */
	breadcrumbs: Breadcrumb[];
	/** The sub text of the page head */
	subText: string;
	/** Additional CSS classes for the header */
	class?: string;
	/** Actions to be displayed in the right side of header */
	actions?: () => JSX.Element;
	/** Bottom content of the header, e.g. switchable tabs */
	bottomContent?: () => JSX.Element;
}

const PageContainerHead = (rawProps: PageContainerHeadProps) => {
	const props = mergeProps(
		{
			class: "",
		},
		rawProps
	);

	return (
		<>
			<header class={`h-full bg-secondary-light rounded-t-xs p-md py-md md:p-xl md:py-lg ${props.class}`}>
				<div class="mx-auto w-full max-w-300 flex flex-col items-start gap-3 md:flex-row md:justify-between md:items-center md:gap-2">
					<div class="flex flex-col gap-2 justify-start min-w-0 w-full md:w-auto">
						<div class="flex gap-4 items-center select-none flex-wrap">
							<For each={props.breadcrumbs}>
								{(crumb, index) => {
									return (
										<>
											<Show when={index() !== 0}>
												<span class="text-xl text-white">&gt;</span>
											</Show>

											<h1
												class={`text-xl ${crumb.url ? "text-primary cursor-pointer" : "text-white"}`}
											>
												{crumb.url ? <Link href={crumb.url}>{crumb.label}</Link> : crumb.label}
											</h1>
										</>
									);
								}}
							</For>
						</div>

						<p class="text-grey text-xs">{props.subText}</p>
					</div>

					<div class="w-full md:w-auto flex flex-wrap gap-2">{props.actions?.()}</div>
				</div>
			</header>
			{props.bottomContent && (
				<div class="bg-secondary-light px-md md:px-xl">
					<div class="mx-auto w-full max-w-300 overflow-x-auto">{props.bottomContent?.()}</div>
				</div>
			)}
		</>
	);
};

export { HeadTab };
export default PageContainerHead;
