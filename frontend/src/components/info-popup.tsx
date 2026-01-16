import { createSignal, JSX, Show } from "solid-js";
import { useClickOutside } from "~/hooks";

interface InfoPopupProps {
	triggerIcon: () => JSX.Element;
	title?: string;
	content: (close: () => void) => JSX.Element;
}

const InfoPopup = (props: InfoPopupProps) => {
	const [showInstructions, setShowInstructions] = createSignal(false);
	const [popupRef, setPopupRef] = createSignal<HTMLDivElement>();
	useClickOutside(popupRef, () => {
		setShowInstructions(false);
	});

	return (
		<div ref={setPopupRef} class="relative inline-block">
			<button
				onClick={(e) => {
					e.stopPropagation();
					setShowInstructions(!showInstructions());
				}}
				class="p-1 rounded hover:bg-white/10 transition-colors cursor-pointer"
				title="Click for verification instructions"
			>
				{props.triggerIcon()}
			</button>
			<Show when={showInstructions()}>
				<div class="absolute z-10 mt-2 p-4 bg-secondary-light border border-white/10 rounded-lg shadow-lg w-80 right-0">
					{props.title && <h4 class="text-white font-semibold mb-2">{props.title}</h4>}
					{props.content(() => setShowInstructions(false))}
				</div>
			</Show>
		</div>
	);
};

export default InfoPopup;
