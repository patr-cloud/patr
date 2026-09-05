import { Accessor, createEffect, createSignal, onCleanup } from "solid-js";

/** How tall a dropdown may grow before its contents scroll. */
const MAX_HEIGHT_PX = 240;
/**
 * The least room worth opening downwards into. Above this the list just scrolls
 * in whatever space is left; below it there are too few visible rows to pick
 * from and flipping above the anchor is the better trade.
 */
const MIN_USABLE_PX = 144;
/** Gap kept between the dropdown and the edge of the viewport. */
const VIEWPORT_MARGIN_PX = 8;

/** Where a portalled dropdown should sit, in viewport coordinates. */
export interface DropdownPosition {
	/** Viewport `top` for a downwards dropdown. */
	top: number;
	left: number;
	width: number;
	/** Viewport `bottom` for an upwards dropdown. */
	bottomOffset: number;
	direction: "down" | "up";
	/** Height cap, so the list scrolls instead of running off-screen. */
	maxHeight: number;
}

/**
 * Positions a dropdown against its anchor, re-measuring while it is open.
 *
 * Opening downwards is strongly preferred. Flipping upwards merely because the
 * full height doesn't fit means a dropdown covers whatever sits above it — in a
 * stack of binding rows that is the row being edited, which is exactly what the
 * user is trying to look at. So it only flips when the space below is too small
 * to show a usable number of rows, and otherwise caps its height and scrolls.
 *
 * Meant to be rendered through a `<Portal>` with `position: fixed`, so no
 * ancestor's `overflow` or stacking context can clip it.
 */
export const createDropdownPosition = (
	anchor: Accessor<HTMLElement | undefined>,
	isOpen: Accessor<boolean>
): Accessor<DropdownPosition> => {
	const [position, setPosition] = createSignal<DropdownPosition>({
		top: 0,
		left: 0,
		width: 0,
		bottomOffset: 0,
		direction: "down",
		maxHeight: MAX_HEIGHT_PX,
	});

	const measure = () => {
		const el = anchor();
		if (!el || typeof window === "undefined") return;

		const rect = el.getBoundingClientRect();
		const spaceBelow = window.innerHeight - rect.bottom - VIEWPORT_MARGIN_PX;
		const spaceAbove = rect.top - VIEWPORT_MARGIN_PX;
		const flip = spaceBelow < MIN_USABLE_PX && spaceAbove > spaceBelow;

		setPosition({
			top: rect.bottom,
			left: rect.left,
			width: rect.width,
			bottomOffset: window.innerHeight - rect.top,
			direction: flip ? "up" : "down",
			maxHeight: Math.max(0, Math.min(MAX_HEIGHT_PX, flip ? spaceAbove : spaceBelow)),
		});
	};

	/**
	 * Scrolls the anchor towards the middle of the viewport when there isn't
	 * room to open downwards. Flipping is the fallback, not the goal — an
	 * upwards dropdown covers whatever the user was just looking at, and near
	 * the bottom of a page there is usually scroll left to spend instead. A
	 * no-op when the page is already scrolled as far as it goes, which is
	 * exactly when flipping is the right answer.
	 */
	const ensureRoomBelow = () => {
		const el = anchor();
		if (!el || typeof window === "undefined") return;
		const rect = el.getBoundingClientRect();
		if (window.innerHeight - rect.bottom - VIEWPORT_MARGIN_PX >= MIN_USABLE_PX) return;
		el.scrollIntoView({ block: "center", behavior: "smooth" });
	};

	createEffect(() => {
		if (!isOpen()) return;
		ensureRoomBelow();
		measure();
		const onMove = () => measure();
		// Capture phase: the anchor may sit inside a scrollable panel whose
		// scroll events never reach the window otherwise.
		window.addEventListener("scroll", onMove, true);
		window.addEventListener("resize", onMove);
		onCleanup(() => {
			window.removeEventListener("scroll", onMove, true);
			window.removeEventListener("resize", onMove);
		});
	});

	return position;
};

export default createDropdownPosition;
