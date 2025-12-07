import { Component, createSignal } from "solid-js";
import { FiCopy, FiCheck } from "solid-icons/fi";

interface CopyableTextFieldProps {
	label: string;
	value: string;
	disabled?: boolean;
}

const CopyableTextField: Component<CopyableTextFieldProps> = (props) => {
	const [copied, setCopied] = createSignal(false);

	const handleCopy = async () => {
		if (props.disabled || !props.value) return;

		try {
			await navigator.clipboard.writeText(props.value);
			setCopied(true);
			setTimeout(() => setCopied(false), 2000);
		} catch (error) {
			console.error("Failed to copy:", error);
		}
	};

	return (
		<div>
			<div class="text-xs text-gray-500 mb-1 select-none">{props.label}</div>
			<div class="flex items-center gap-2 bg-black/20 rounded px-3 py-2">
				<span class="text-sm text-gray-300 flex-1 truncate font-mono">
					{props.value || "Not available"}
				</span>
				<button
					onClick={handleCopy}
					class={`transition-colors text-gray-400 ${copied() ? "" : "hover:text-white"}`}
					title={copied() ? "Copied!" : `Copy ${props.label}`}
					disabled={props.disabled || !props.value}
				>
					{copied() ? <FiCheck size={14} /> : <FiCopy size={14} />}
				</button>
			</div>
		</div>
	);
};

export default CopyableTextField;
