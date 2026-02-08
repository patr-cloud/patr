import { NavigateOptions, SearchParams } from "@solidjs/router";
import { JSX, mergeProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor, SetSearchParams } from "~/utils/types";

export interface HeadTabProps {
	/** Additional CSS classes for the tab container */
	class?: MaybeAccessor<string>;
	/** Additional CSS classes for the tab buttons */
	buttonClass?: MaybeAccessor<string>;
	/** Search Params */
	searchParams: Partial<SearchParams>;
	/** Tab */
	tab: MaybeAccessor<string>;
	/** Set Search Params */
	setSearchParams: (params: SetSearchParams, options?: Partial<NavigateOptions>) => void;
	/** Tab Items */
	tabItems: Array<{
		/** Button Class */
		class?: MaybeAccessor<string>;
		/** The label of the tab */
		label: string;
		/** The value of the tab */
		value: string;
		/** OnClick handler for the tab */
		onClick?: (
			value: string,
			setSearchParams?: (params: SetSearchParams, options?: Partial<NavigateOptions>) => void
		) => void;
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
		<div class={`w-full text-white flex gap-4 ${get(props.class)}`}>
			{props.tabItems.map((item) => (
				<button
					onClick={() => {
						if (item.onClick) {
							item.onClick(item.value, props.setSearchParams);
						}
					}}
					class={`pb-2 px-2 border-b-2 ${
						get(props.tab) === item.value ? "border-primary" : "border-none"
					} ${get(props.buttonClass)} ${get(item.class) || ""}`}
				>
					{item.label}
				</button>
			))}
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
	subText: JSX.Element | string;
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
			<header
				class={`h-full bg-secondary-light flex justify-between items-center gap-2 rounded-t-xs p-xl py-lg ${props.class}`}
			>
				<div class="flex flex-col gap-2 justify-start">
					<div class="flex gap-4 items-center select-none">
						{props.breadcrumbs.map((crumb, index) => {
							if (index === 0) {
								return (
									<h1 class={`text-xl ${crumb.url ? "text-primary cursor-pointer" : "text-white"}`}>
										{crumb.url ? <a href={crumb.url}>{crumb.label}</a> : crumb.label}
									</h1>
								);
							} else {
								return (
									<>
										{index !== 0 && <span class="text-xl text-white">&gt;</span>}
										<h2 class={`${crumb.url ? "text-primary cursor-pointer" : "text-white"} text-md`}>
											{crumb.url ? <a href={crumb.url}>{crumb.label}</a> : crumb.label}
										</h2>
									</>
								);
							}
						})}
					</div>

					<p class="text-grey text-xs">{props.subText}</p>
				</div>

				<div>{props.actions?.()}</div>
			</header>
			{props.bottomContent && <div class="bg-secondary-light px-xl">{props.bottomContent?.()}</div>}
		</>
	);
};

export { HeadTab };
export default PageContainerHead;
