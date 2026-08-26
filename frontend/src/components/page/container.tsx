import { mergeProps, ParentProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface PageContainerProps {
	/** Additional Classes to add */
	class?: MaybeAccessor<string>;
	/**
	 * Bound the page to the viewport instead of growing with its content, so
	 * that panels inside it can scroll on their own. A prop rather than a class
	 * the caller passes, because the default is `min-height` — two competing
	 * height utilities on one element resolve by stylesheet order, not by the
	 * order they appear in the class list, so overriding it from outside is a
	 * coin flip.
	 */
	fillViewport?: MaybeAccessor<boolean>;
}

const PageContainer = (rawProps: ParentProps<PageContainerProps>) => {
	const props = mergeProps({}, rawProps);

	const heightClass = () =>
		get(props.fillViewport)
			? "h-full min-h-0 overflow-hidden"
			: "min-h-[calc(100vh-56px)] md:min-h-[calc(100vh-64px)]";

	return (
		<div
			class={`${heightClass()} ${get(props.class) || ""} bg-secondary p-xs md:p-sm md:pl-0 md:ml-sm flex flex-col`}
		>
			{props.children}
		</div>
	);
};

export default PageContainer;
