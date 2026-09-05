import { mergeProps, ParentProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface PageContainerBodyProps {
	/**
	 * Additional Classes for the body.
	 */
	class?: MaybeAccessor<string>;
}

/*
 * `flex-1 min-h-0`, not `h-full`: as a flex item, `height: 100%` gets shrunk to
 * fit beside the head and collapses to a few pixels once the page is bound to
 * the viewport. `min-h-0` is what lets it scroll instead of growing.
 */
const PageContainerBody = (rawProps: ParentProps<PageContainerBodyProps>) => {
	const props = mergeProps(
		{
			class: "",
		},
		rawProps
	);

	return (
		<section class="min-h-0 bg-secondary-dark p-sm md:p-md rounded-b-xs text-white flex-1 text-sm relative flex flex-col">
			<div class={`mx-auto w-full max-w-300 flex-1 min-h-0 ${get(props.class)}`}>{props.children}</div>
		</section>
	);
};

export default PageContainerBody;
