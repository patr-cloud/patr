import { FiCheck, FiCopy } from "solid-icons/fi";
import { createSignal, mergeProps, ParentProps } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface CopyButtonProps {
	text: string;
	class?: MaybeAccessor<string | undefined>;
	timeout?: number;
	onClick?: (event: MouseEvent & { currentTarget: HTMLButtonElement }) => void;
}

const CopyButton = (rawProps: ParentProps<CopyButtonProps>) => {
	const props = mergeProps(
		{
			class: "",
			timeout: 2000,
		},
		rawProps
	);
	const [copied, setCopied] = createSignal(false);

	const handleCopy = async (e: MouseEvent & { currentTarget: HTMLButtonElement }) => {
		e.stopPropagation();
		try {
			await navigator.clipboard.writeText(props.text);
			setCopied(true);
			setTimeout(() => setCopied(false), props.timeout);
		} catch (error) {
			console.error("Failed to copy:", error);
		} finally {
			if (props.onClick) {
				props.onClick(e);
			}
		}
	};

	return (
		<button
			onClick={handleCopy}
			class={`ml-2 p-1 rounded hover:bg-white/10 transition-colors ${get(props.class)}`}
			title={copied() ? "Copied!" : "Copy ID"}
		>
			{copied() ? <FiCheck size={14} class="text-gray-400" /> : <FiCopy size={14} class="text-gray-400" />}
		</button>
	);
};

export default CopyButton;
