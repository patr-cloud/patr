import { Accessor, createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { FiCheck, FiX } from "solid-icons/fi";
import { passwordStrength } from "~/utils/validation";
import { loadEstimator } from "~/utils/password-estimator";

interface PasswordStrengthProps {
	/** Accessor for the current password value. */
	password: Accessor<string>;
	/** Accessor for the element the panel anchors to (the field wrapper). */
	anchor: Accessor<HTMLElement | undefined>;
	/** Accessor for whether the field is focused. */
	show: Accessor<boolean>;
}

const SEGMENT_COUNT = 4;
const PANEL_WIDTH = 260;
const GAP = 12;
const MD_BREAKPOINT = 768;

interface Position {
	placement: "right" | "above";
	left: number;
	width: number;
	top?: number;
	bottom?: number;
}

const bgForColor = (color: "error" | "warning" | "success") =>
	color === "error" ? "bg-error" : color === "warning" ? "bg-warning" : "bg-success";

const textForColor = (color: "error" | "warning" | "success") =>
	color === "error" ? "text-error" : color === "warning" ? "text-warning" : "text-success";

/**
 * A floating panel anchored to a password field showing a stepped strength
 * meter and a live requirements checklist. Placed to the right of the field on
 * viewports >= md, above it on narrow viewports. Shown while the field is
 * focused and non-empty. Purely informational — rendered via a Portal with
 * pointer-events disabled so it never intercepts clicks.
 */
const PasswordStrength = (props: PasswordStrengthProps) => {
	const visible = () => props.show() && props.password().length > 0;

	// zxcvbn scorer, lazy-loaded on first use. Until it resolves, `strength`
	// falls back to the length heuristic inside `passwordStrength`.
	const [scorer, setScorer] = createSignal<(password: string) => number>();
	const strength = createMemo(() => {
		const score = scorer()?.(props.password());
		return passwordStrength(props.password(), score);
	});

	const [position, setPosition] = createSignal<Position>();

	const updatePosition = () => {
		const el = props.anchor();
		if (!el || typeof window === "undefined") return;
		const rect = el.getBoundingClientRect();
		const wideEnough = window.innerWidth >= MD_BREAKPOINT;
		const roomOnRight = rect.right + GAP + PANEL_WIDTH <= window.innerWidth;
		if (wideEnough && roomOnRight) {
			setPosition({ placement: "right", left: rect.right + GAP, top: rect.top, width: PANEL_WIDTH });
		} else {
			setPosition({
				placement: "above",
				left: rect.left,
				bottom: window.innerHeight - rect.top + GAP,
				width: rect.width,
			});
		}
	};

	createEffect(() => {
		if (!visible()) return;
		// Kick off the zxcvbn load the first time the panel is shown; idempotent.
		if (!scorer()) void loadEstimator().then((fn) => setScorer(() => fn));
		updatePosition();
		const onMove = () => updatePosition();
		window.addEventListener("scroll", onMove, true);
		window.addEventListener("resize", onMove);
		onCleanup(() => {
			window.removeEventListener("scroll", onMove, true);
			window.removeEventListener("resize", onMove);
		});
	});

	const panelStyle = () => {
		const p = position();
		if (!p) return {};
		return {
			position: "fixed" as const,
			left: `${p.left}px`,
			width: `${p.width}px`,
			...(p.placement === "right" ? { top: `${p.top}px` } : { bottom: `${p.bottom}px` }),
		};
	};

	const tierLabel = () => {
		const t = strength().tier;
		return t.charAt(0).toUpperCase() + t.slice(1);
	};

	return (
		<Show when={visible() && position()}>
			<Portal>
				<div
					style={panelStyle()}
					class="z-50 pointer-events-none p-4 bg-secondary-light border border-white/10 rounded-lg shadow-lg"
				>
					<div class="flex items-center justify-between mb-2">
						<span class="text-xxs text-grey uppercase tracking-wide">Password strength</span>
						<span class={`text-xs font-medium ${textForColor(strength().color)}`}>{tierLabel()}</span>
					</div>
					<div class="flex gap-1 mb-3">
						<For each={Array.from({ length: SEGMENT_COUNT })}>
							{(_, index) => (
								<div
									class={`h-1 flex-1 rounded-full ${
										index() < strength().segments
											? bgForColor(strength().color)
											: "bg-secondary-medium"
									}`}
								/>
							)}
						</For>
					</div>
					<ul class="flex flex-col gap-1">
						<For each={strength().requirements}>
							{(req) => (
								<li
									class={`flex items-center gap-2 text-xxs ${req.met ? "text-success" : "text-grey"}`}
								>
									{req.met ? (
										<FiCheck size={12} class="text-success" />
									) : (
										<FiX size={12} class="text-grey" />
									)}
									<span>{req.label}</span>
								</li>
							)}
						</For>
					</ul>
				</div>
			</Portal>
		</Show>
	);
};

export default PasswordStrength;
