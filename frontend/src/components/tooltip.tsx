import { JSX, ParentProps, createSignal, mergeProps, onCleanup } from "solid-js";
import { MaybeAccessor } from "~/utils/types";
import { get } from "~/utils/func";

interface TooltipProps {
	/**
	 * The content to display inside the tooltip
	 */
	content: MaybeAccessor<string | JSX.Element>;
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
	/**
	 * Offset from cursor in pixels
	 * @default 20
	 */
	xOffset?: number;
	/**
	 * Offset from cursor in pixels
	 * @default 0
	 */
	yOffset?: number;
}

const Tooltip = (rawProps: ParentProps<TooltipProps>) => {
	const props = mergeProps(
		{
			delay: 200,
			class: "",
			tooltipClass: "",
			xOffset: 20,
			yOffset: 0,
		},
		rawProps
	);

	const [isVisible, setIsVisible] = createSignal(false);
	const [mousePosition, setMousePosition] = createSignal({ x: 0, y: 0 });
	const [tooltipRef, setTooltipRef] = createSignal<HTMLDivElement>();
	let timeoutId: number | undefined;

	const handleMouseMove = (e: MouseEvent) => {
		setMousePosition({ x: e.clientX, y: e.clientY });
	};

	const handleMouseEnter = (e: MouseEvent) => {
		setMousePosition({ x: e.clientX, y: e.clientY });
		timeoutId = window.setTimeout(() => {
			setIsVisible(true);
		}, props.delay);
	};

	const handleMouseLeave = () => {
		if (timeoutId) {
			clearTimeout(timeoutId);
			timeoutId = undefined;
		}
		setIsVisible(false);
	};

	onCleanup(() => {
		if (timeoutId) {
			clearTimeout(timeoutId);
		}
	});

	const tooltipStyle = () => {
		const tooltip = tooltipRef();
		if (!tooltip) {
			return {
				left: `${mousePosition().x + props.xOffset}px`,
				top: `${mousePosition().y + props.yOffset}px`,
			};
		}

		const tooltipRect = tooltip.getBoundingClientRect();
		const viewportWidth = window.innerWidth;
		const viewportHeight = window.innerHeight;

		let x = mousePosition().x + props.xOffset;
		let y = mousePosition().y + props.yOffset;

		// Adjust horizontal position if tooltip would overflow right edge
		if (x + tooltipRect.width > viewportWidth) {
			x = mousePosition().x - tooltipRect.width - props.xOffset;
		}

		// Adjust vertical position if tooltip would overflow bottom edge
		if (y + tooltipRect.height > viewportHeight) {
			y = mousePosition().y - tooltipRect.height - props.yOffset;
		}

		// Ensure tooltip doesn't go off left edge
		if (x < 0) {
			x = props.xOffset;
		}

		// Ensure tooltip doesn't go off top edge
		if (y < 0) {
			y = props.yOffset;
		}

		return {
			left: `${x}px`,
			top: `${y}px`,
		};
	};

	const handleFocus = () => {
		timeoutId = window.setTimeout(() => {
			setIsVisible(true);
		}, props.delay);
	};

	const handleBlur = () => {
		if (timeoutId) {
			clearTimeout(timeoutId);
			timeoutId = undefined;
		}
		setIsVisible(false);
	};

	return (
		<div class={`inline-block ${get(props.class) ?? ""}`}>
			<div
				onMouseEnter={handleMouseEnter}
				onMouseLeave={handleMouseLeave}
				onMouseMove={handleMouseMove}
				onFocusIn={handleFocus}
				onFocusOut={handleBlur}
			>
				{props.children}
			</div>
			{isVisible() && (
				<div ref={setTooltipRef} class="fixed z-50 pointer-events-none" style={tooltipStyle()} role="tooltip">
					<div
						class={`bg-secondary-dark text-white px-sm py-xs rounded-xs text-sm whitespace-nowrap shadow-lg ${get(props.tooltipClass)}`}
					>
						{get(props.content)}
					</div>
				</div>
			)}
		</div>
	);
};

export default Tooltip;
