import { mergeProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface CheckboxProps {
	checked: MaybeAccessor<boolean>;
	onChange?: (checked: boolean) => void;
	disabled?: MaybeAccessor<boolean>;
	class?: MaybeAccessor<string | undefined>;
	label?: string;
	name?: string;
	size?: "sm" | "md" | "lg";
}

const Checkbox = (rawProps: CheckboxProps) => {
	const props = mergeProps(
		{
			disabled: false as MaybeAccessor<boolean>,
			class: "" as MaybeAccessor<string | undefined>,
			size: "md" as const,
		},
		rawProps
	);

	const sizeClasses = () => {
		switch (props.size) {
			case "sm":
				return { box: "w-3.5 h-3.5", icon: "w-2.5 h-2.5" };
			case "lg":
				return { box: "w-6 h-6", icon: "w-4 h-4" };
			case "md":
			default:
				return { box: "w-[18px] h-[18px]", icon: "w-3 h-3" };
		}
	};

	const isChecked = () => get(props.checked);
	const isDisabled = () => get(props.disabled);

	return (
		<label
			class={`inline-flex items-center gap-2 select-none ${
				isDisabled() ? "cursor-not-allowed opacity-50" : "cursor-pointer"
			} ${get(props.class) ?? ""}`}
		>
			<input
				type="checkbox"
				checked={isChecked()}
				disabled={isDisabled()}
				name={props.name}
				onChange={() => {
					if (!isDisabled() && props.onChange) {
						props.onChange(!isChecked());
					}
				}}
				class="sr-only"
			/>
			<span
				class={`
					${sizeClasses().box}
					inline-flex items-center justify-center shrink-0
					rounded-[3px] border transition-all duration-200
					${isChecked() ? "bg-primary border-primary" : "bg-transparent border-grey hover:border-primary"}
				`}
			>
				{isChecked() && (
					<svg
						class={`${sizeClasses().icon} text-secondary`}
						viewBox="0 0 12 12"
						fill="none"
						stroke="currentColor"
						stroke-width="2"
						stroke-linecap="round"
						stroke-linejoin="round"
					>
						<path d="M2 6l3 3 5-5" />
					</svg>
				)}
			</span>
			{props.label && (
				<span class={`text-sm ${isDisabled() ? "text-disabled" : "text-white"}`}>{props.label}</span>
			)}
		</label>
	);
};

export default Checkbox;
