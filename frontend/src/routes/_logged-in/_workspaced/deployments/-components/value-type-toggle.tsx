import { For } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

/** Which side of the union an environment variable's value sits on. */
export type ValueType = "string" | "secret";

interface ValueTypeToggleProps {
	/** The currently selected side. */
	value: MaybeAccessor<ValueType>;
	/** Fires with the side the user picked. */
	onChange: (value: ValueType) => void;
	/** Whether the toggle can be operated. */
	disabled?: MaybeAccessor<boolean>;
	/** Additional class for the container. */
	class?: MaybeAccessor<string>;
}

const OPTIONS: Array<{ value: ValueType; label: string }> = [
	{ value: "string", label: "String" },
	{ value: "secret", label: "Secret" },
];

/** A two-segment switch between a literal value and a secret reference. */
const ValueTypeToggle = (props: ValueTypeToggleProps) => {
	const isDisabled = () => get(props.disabled) ?? false;

	return (
		<div
			role="radiogroup"
			class={`flex items-stretch self-stretch shrink-0 p-0.5 rounded-xs bg-secondary-light ${
				get(props.class) ?? ""
			}`}
		>
			<For each={OPTIONS}>
				{(option) => (
					<button
						type="button"
						role="radio"
						aria-checked={get(props.value) === option.value}
						disabled={isDisabled()}
						onClick={() => props.onChange(option.value)}
						class={`px-3 text-xs rounded-xs transition-colors ${
							get(props.value) === option.value
								? "bg-secondary-dark text-white font-medium"
								: "text-grey hover:text-white"
						} ${isDisabled() ? "opacity-50 cursor-not-allowed" : "cursor-pointer"}`}
					>
						{option.label}
					</button>
				)}
			</For>
		</div>
	);
};

export default ValueTypeToggle;
