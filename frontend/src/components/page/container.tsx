import { mergeProps, ParentProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface PageContainerProps {
	/** Additional Classes to add */
	class?: MaybeAccessor<string>;
}

const PageContainer = (rawProps: ParentProps<PageContainerProps>) => {
	const props = mergeProps({}, rawProps);

	return (
		<div class={`min-h-[calc(100vh-56px)] md:min-h-[calc(100vh-64px)] ${get(props.class) || ""} bg-secondary p-xs md:p-sm md:pl-0 md:ml-sm flex flex-col`}>
			{props.children}
		</div>
	);
};

export default PageContainer;
