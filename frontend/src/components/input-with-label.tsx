import { JSX, mergeProps } from "solid-js";
import InputLabel from "~/components/input-label";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface InputWithLabelProps {
	/** Label text */
	label: string;
	/** For attribute on the label, should match the input's name/id */
	for?: string;
	/** The input (or input-like element) to render alongside the label */
	children: JSX.Element;
	/** Optional helper text under the label */
	comments?: string;
	/** Additional classes for the outer wrapper */
	class?: MaybeAccessor<string>;
	/** Additional classes for the label wrapper */
	labelClass?: MaybeAccessor<string>;
	/** Additional classes for the input wrapper */
	inputClass?: MaybeAccessor<string>;
}

/**
 * Label + input row that stacks vertically on mobile and aligns
 * horizontally (label left, input right) on md+ screens.
 */
const InputWithLabel = (rawProps: InputWithLabelProps) => {
	const props = mergeProps({ class: "", labelClass: "", inputClass: "" }, rawProps);

	return (
		<div class={`w-full flex flex-col md:flex-row md:items-center gap-2 md:gap-4 ${get(props.class) ?? ""}`}>
			<InputLabel
				for={props.for}
				label={props.label}
				comments={props.comments}
				parentClass={`md:flex-2 ${get(props.labelClass) ?? ""}`}
			/>
			<div class={`w-full md:flex-10 [&>*]:w-full ${get(props.inputClass) ?? ""}`}>{props.children}</div>
		</div>
	);
};

export default InputWithLabel;
