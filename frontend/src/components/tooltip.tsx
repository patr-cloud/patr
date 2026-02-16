import { JSX, ParentProps, createSignal, mergeProps, onCleanup, onMount } from "solid-js";
import { MaybeAccessor } from "~/utils/types";
import { get } from "~/utils/func";

interface TooltipProps {
	/**
	 * The content to display inside the tooltip
	 */
	content: MaybeAccessor<string | JSX.Element>;
	/**
	 * Position of the tooltip relative to the trigger element
	 * @default "top"
	 */
	position?: "top" | "bottom" | "left" | "right";
	/**
	 * Additional classes for the wrapper container
	 */
	class?: MaybeAccessor<string | undefined>;
	/**
	 * Additional classes for the tooltip bubble itself
	 */
	tooltipClass?: MaybeAccessor<string | undefined>;
	/**
	 * Delay in milliseconds before showing the tooltip
	 * @default 200
	 */
	delay?: number;
}

const Tooltip = (rawProps: ParentProps<TooltipProps>) => {
	const props = mergeProps(
		{
			position: "top" as const,
			delay: 200,
			class: "",
			tooltipClass: "bg-secondary-dark text-white px-sm py-xs rounded-xs text-sm whitespace-nowrap shadow-lg",
		},
		rawProps
	);

	const [isVisible, setIsVisible] = createSignal(false);
	const [triggerRef, setTriggerRef] = createSignal<HTMLDivElement>();
	let timeoutId: number | undefined;

	const handleMouseEnter = () => {
		timeoutId = window.setTimeout(() => {
			setIsVisible(true);
		}, props.delay);
	};

	const handleMouseLeave = () => {
		if (timeoutId) {
			clearTimeout(timeoutId);
		}
		setIsVisible(false);
	};

	onCleanup(() => {
		if (timeoutId) {
			clearTimeout(timeoutId);
		}
	});

	const positionClasses = () => {
		switch (props.position) {
			case "top":
				return "bottom-full left-1/2 -translate-x-1/2 mb-2";
			case "bottom":
				return "top-full left-1/2 -translate-x-1/2 mt-2";
			case "left":
				return "right-full top-1/2 -translate-y-1/2 mr-2";
			case "right":
				return "left-full top-1/2 -translate-y-1/2 ml-2";
		}
	};

	const arrowClasses = () => {
		switch (props.position) {
			case "top":
				return "top-full left-1/2 -translate-x-1/2 border-l-transparent border-r-transparent border-b-transparent border-t-secondary-dark";
			case "bottom":
				return "bottom-full left-1/2 -translate-x-1/2 border-l-transparent border-r-transparent border-t-transparent border-b-secondary-dark";
			case "left":
				return "left-full top-1/2 -translate-y-1/2 border-t-transparent border-b-transparent border-r-transparent border-l-secondary-dark";
			case "right":
				return "right-full top-1/2 -translate-y-1/2 border-t-transparent border-b-transparent border-l-transparent border-r-secondary-dark";
		}
	};

	return (
		<div class={`relative inline-block ${get(props.class) ?? ""}`} ref={setTriggerRef}>
			<div onMouseEnter={handleMouseEnter} onMouseLeave={handleMouseLeave}>
				{props.children}
			</div>
			{isVisible() && (
				<div class={`absolute z-50 ${positionClasses()} pointer-events-none`} role="tooltip">
					<div class={get(props.tooltipClass) ?? ""}>{get(props.content)}</div>
					<div class={`absolute w-0 h-0 border-4 ${arrowClasses()}`} />
				</div>
			)}
		</div>
	);
};

export default Tooltip;
