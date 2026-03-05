import { JSX } from "solid-js";
import { MaybeAccessor } from "~/utils/types";
import Input from "~/components/input";

export interface InputDropdownOption {
	/** The Label of to be rendered */
	label: string;
	/** The Value of the option, e.g. id, index etc */
	value: string;
}

interface InputDropdownProps {
	/** Dropdown Options */
	options: MaybeAccessor<InputDropdownOption[]>;
	/** On Select Option */
	onSelect: (value: string) => void;
	/** Additional Classes for the input.  */
	class?: MaybeAccessor<string>;
	/** The placeholder text for the input */
	placeholder?: string;
	/** The currently selected value */
	value?: MaybeAccessor<string | undefined>;
	/** The Color Variant of the input */
	styleVariant?: "light" | "medium" | "dark";
	/** Whether the input is disabled or not */
	disabled?: MaybeAccessor<boolean>;
	/**
	 * Specifies whether the form field needs to be filled in before it can
	 * be submitted, doesn't use javascript.
	 */
	required?: boolean;
	/** The ID of the input, this is used to identify the input in the DOM. */
	id?: string;
	/** The name of the input, this is used to identify the input in a form submission. */
	name?: string;
	/** @deprecated Ignored — chevron is rendered internally by Input. */
	endIcon?: () => JSX.Element;
	/** @deprecated Ignored. */
	onClickEndIcon?: () => void;
}

const InputDropdown = (props: InputDropdownProps) => (
	<Input
		suggestions={props.options}
		allowCustomValue={false}
		onSelect={props.onSelect}
		value={props.value}
		class={props.class}
		placeholder={props.placeholder}
		styleVariant={props.styleVariant}
		disabled={props.disabled}
		required={props.required}
		id={props.id}
		name={props.name}
	/>
);

export default InputDropdown;
