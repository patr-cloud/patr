import { FiCheck, FiCopy } from "solid-icons/fi";
import { createSignal, mergeProps, Show } from "solid-js";
import { CopyableFieldVariant, CopyableFieldVariantEnum } from "~/utils/color";
import { get, variantBgClass } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface CopyableFieldProps {
	/** The text to display and copy */
	value: MaybeAccessor<string>;
	/** Optional label rendered above the field */
	label?: MaybeAccessor<string>;
	/** Visual variant. Default: CopyableFieldVariant.Input */
	variant?: CopyableFieldVariantEnum;
	/** Background style, uses variantBgClass(). Default: "light" */
	styleVariant?: "light" | "medium" | "dark";
	/** Whether copying is disabled */
	disabled?: MaybeAccessor<boolean>;
	/** Additional classes for the outer container */
	class?: MaybeAccessor<string>;
	/** Additional classes for the value text */
	innerClass?: MaybeAccessor<string>;
	/** Copy button location */
	buttonPosition?: "start" | "end";
	/** Callback after successful copy (e.g., show toast) */
	onCopy?: () => void;
}

const CopyableField = (rawProps: CopyableFieldProps) => {
	const props = mergeProps(
		{
			variant: CopyableFieldVariant.Input,
			styleVariant: "light",
			disabled: false,
			buttonPosition: "end",
		},
		rawProps
	);

	const [copied, setCopied] = createSignal(false);

	const handleCopy = async (e: MouseEvent) => {
		e.stopPropagation();

		if (get(props.disabled) || !get(props.value)) return;
		try {
			await navigator.clipboard.writeText(get(props.value));
			setCopied(true);
			setTimeout(() => setCopied(false), 2000);
			props.onCopy?.();
		} catch (error) {
			console.error("Failed to copy:", error);
		}
	};

	const copyButton = () => (
		<button
			type="button"
			onClick={handleCopy}
			class={`p-1 rounded hover:bg-white/10 transition-colors ${copied() ? "" : "hover:text-white"}`}
			title={copied() ? "Copied!" : "Copy"}
			disabled={get(props.disabled) || !get(props.value)}
		>
			{copied() ? <FiCheck size={14} class="text-gray-400" /> : <FiCopy size={14} class="text-gray-400" />}
		</button>
	);

	return (
		<div class={get(props.class)}>
			<Show when={get(props.label)}>
				<div class="text-xs text-gray-500 mb-1 select-none">{get(props.label)}</div>
			</Show>
			<div
				class={
					props.variant === CopyableFieldVariant.Input
						? `rounded-xs flex items-center border border-secondary-medium ${variantBgClass(get(props.styleVariant))} py-xs px-md min-w-0`
						: `flex items-center gap-0.5`
				}
			>
				{props.buttonPosition === "start" && copyButton()}
				<span
					class={`flex-1 min-w-0 text-sm text-gray-400 ${
						props.variant === CopyableFieldVariant.Input
							? `overflow-x-auto whitespace-nowrap ${get(props.innerClass) || ""}`
							: `truncate font-mono ${get(props.innerClass) || ""}`
					}`}
					style={props.variant === CopyableFieldVariant.Input ? { "scrollbar-width": "none" } : undefined}
				>
					{get(props.value) || "Not available"}
				</span>
				{props.buttonPosition === "end" && copyButton()}
			</div>
		</div>
	);
};

export default CopyableField;
