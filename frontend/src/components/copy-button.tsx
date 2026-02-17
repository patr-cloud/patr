import { FiCheck, FiCopy } from "solid-icons/fi";
import { createSignal } from "solid-js";

const CopyButton = (props: { text: string }) => {
	const [copied, setCopied] = createSignal(false);

	const handleCopy = async (e: MouseEvent) => {
		e.stopPropagation(); // Prevent row click navigation
		try {
			await navigator.clipboard.writeText(props.text);
			setCopied(true);
			setTimeout(() => setCopied(false), 2000);
		} catch (error) {
			console.error("Failed to copy:", error);
		}
	};

	return (
		<button
			onClick={handleCopy}
			class="ml-2 p-1 rounded hover:bg-white/10 transition-colors"
			title={copied() ? "Copied!" : "Copy ID"}
		>
			{copied() ? <FiCheck size={14} class="text-gray-400" /> : <FiCopy size={14} class="text-gray-400" />}
		</button>
	);
};

export default CopyButton;
