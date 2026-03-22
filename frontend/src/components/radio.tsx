import { mergeProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface RadioProps {
	checked: MaybeAccessor<boolean>;
	onChange?: () => void;
	disabled?: MaybeAccessor<boolean>;
	class?: MaybeAccessor<string | undefined>;
	label?: string;
	name?: string;
	value?: string;
	size?: "sm" | "md" | "lg";
}

const Radio = (rawProps: RadioProps) => {
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
				return { outer: "w-3.5 h-3.5", inner: "w-1.5 h-1.5" };
			case "lg":
				return { outer: "w-6 h-6", inner: "w-3 h-3" };
			case "md":
			default:
				return { outer: "w-[18px] h-[18px]", inner: "w-2 h-2" };
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
				type="radio"
				checked={isChecked()}
				disabled={isDisabled()}
				name={props.name}
				value={props.value}
				onChange={() => {
					if (!isDisabled() && props.onChange) {
						props.onChange();
					}
				}}
				class="sr-only"
			/>
			<span
				class={`
					${sizeClasses().outer}
					inline-flex items-center justify-center shrink-0
					rounded-full border transition-all duration-200
					${isChecked() ? "border-primary" : "border-grey hover:border-primary"}
				`}
			>
				<span
					class={`
						${sizeClasses().inner}
						rounded-full transition-all duration-200
						${isChecked() ? "bg-primary scale-100" : "bg-transparent scale-0"}
					`}
				/>
			</span>
			{props.label && (
				<span class={`text-sm ${isDisabled() ? "text-disabled" : "text-white"}`}>{props.label}</span>
			)}
		</label>
	);
};

export default Radio;
