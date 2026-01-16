import { mergeProps } from "solid-js";
import { Color } from "~/utils/color";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface ToggleSwitchProps {
	/**
	 * The current value of the toggle (checked/unchecked)
	 */
	checked: MaybeAccessor<boolean>;
	/**
	 * Called when the toggle state changes
	 */
	onChange?: (checked: boolean) => void;
	/**
	 * Whether the toggle is disabled
	 */
	disabled?: boolean;
	/**
	 * The color of the toggle when checked, defaults to Color.Primary
	 */
	color?: Color;
	/**
	 * Additional classes for the container
	 */
	class?: MaybeAccessor<string | undefined>;
	/**
	 * Label text to display next to the toggle
	 */
	label?: string;
	/**
	 * Size of the toggle switch
	 */
	size?: "sm" | "md" | "lg";
}

const ToggleSwitch = (rawProps: ToggleSwitchProps) => {
	const props = mergeProps(
		{
			disabled: false,
			color: Color.Primary,
			class: "",
			size: "md" as const,
		},
		rawProps
	);

	const handleToggle = () => {
		if (!props.disabled && props.onChange) {
			props.onChange(!get(props.checked));
		}
	};

	const sizeClasses = () => {
		switch (props.size) {
			case "sm":
				return {
					container: "w-8 h-4",
					circle: "w-3 h-3",
					translate: "translate-x-4",
				};
			case "lg":
				return {
					container: "w-14 h-7",
					circle: "w-6 h-6",
					translate: "translate-x-7",
				};
			case "md":
			default:
				return {
					container: "w-11 h-4",
					circle: "w-5 h-5",
					translate: "translate-x-6",
				};
		}
	};

	const isChecked = () => get(props.checked);

	return (
		<div class={`flex items-center gap-2 ${get(props.class) ?? ""}`}>
			<button
				type="button"
				role="switch"
				aria-checked={isChecked()}
				disabled={props.disabled}
				onClick={handleToggle}
				class={`
					${sizeClasses().container}
					relative inline-flex items-center rounded-full
					transition-colors duration-200 ease-in-out
					focus:outline-none focus:ring-2 focus:ring-offset-2 focus:ring-${props.color}
					disabled:opacity-50 disabled:cursor-not-allowed
					${isChecked() ? `bg-${props.color} hover:opacity-90` : "bg-grey hover:bg-grey/80"}
					${!props.disabled ? "cursor-pointer" : ""}
				`}
			>
				<span
					class={`
						${sizeClasses().circle}
						inline-block rounded-full bg-white shadow-lg
						transform transition-transform duration-200 ease-in-out
						${isChecked() ? sizeClasses().translate : ""}
					`}
				/>
			</button>
			{props.label && (
				<span
					class={`
						text-sm font-medium
						${props.disabled ? "text-disabled" : "text-primary"}
					`}
				>
					{props.label}
				</span>
			)}
		</div>
	);
};

export default ToggleSwitch;
