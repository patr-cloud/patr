import { mergeProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface LabelProps {
	/** Label text */
	label: string;
	/** For attribute to associate the label with an input */
	for?: string;
	/** Additional classes for styling the parent div */
	parentClass?: MaybeAccessor<string>;
	/** Additional classes for styling the label */
	labelClass?: MaybeAccessor<string>;
	/** Additional comments */
	comments?: string;
	/** Id attribute for the label */
	id?: string;
}

const Label = (rawProps: LabelProps) => {
	const props = mergeProps(
		{
			parentClass: "",
			labelClass: "",
			for: undefined,
		},
		rawProps
	);

	return (
		<div class={`flex flex-col ${get(props.parentClass) ?? ""}`}>
			<label id={props.id} class={get(props.labelClass)} for={props.for}>
				{props.label}
			</label>
			{props.comments && <small class="text-xxs text-grey">{props.comments}</small>}
		</div>
	);
};

export default Label;
