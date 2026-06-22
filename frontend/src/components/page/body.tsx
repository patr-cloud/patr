import { mergeProps, ParentProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface PageContainerBodyProps {
	/**
	 * Additional Classes for the body.
	 */
	class?: MaybeAccessor<string>;
}

const PageContainerBody = (rawProps: ParentProps<PageContainerBodyProps>) => {
	const props = mergeProps(
		{
			class: "",
		},
		rawProps
	);

	return (
		<section class="h-full bg-secondary-dark p-sm md:p-md rounded-b-xs text-white flex-1 text-sm relative flex flex-col">
			<div class={`mx-auto w-full max-w-300 flex-1 ${get(props.class)}`}>{props.children}</div>
		</section>
	);
};

export default PageContainerBody;
