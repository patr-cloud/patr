import { mergeProps, Show } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface UsageBarProps {
	/** The amount used, in the same unit as `max` (bytes, for registry storage). */
	value: MaybeAccessor<number>;
	/**
	 * The optional maximum/limit. When set, the bar fills to `value / max` and
	 * takes a traffic-light color as it approaches the limit. When absent (no
	 * quota configured yet) the bar is a neutral full track — a visual anchor for
	 * the figure shown alongside it.
	 */
	max?: MaybeAccessor<number | undefined>;
	/** Additional classes for the outer container. */
	class?: MaybeAccessor<string>;
}

/**
 * A slim horizontal usage bar. With a `max` it renders a proportional,
 * traffic-light-colored fill (green → amber → red as it fills); without one it
 * renders a neutral full track, ready to become a real gauge once registry
 * quotas land.
 */
const UsageBar = (rawProps: UsageBarProps) => {
	const props = mergeProps({ class: "" }, rawProps);

	const max = () => get(props.max);
	const hasLimit = () => {
		const limit = max();
		return limit !== undefined && limit > 0;
	};

	const percent = () => {
		const limit = max();
		if (limit === undefined || limit <= 0) return 100;
		return Math.min(100, Math.max(0, (get(props.value) / limit) * 100));
	};

	const fillColor = () => {
		if (!hasLimit()) return "bg-primary/60";
		const pct = percent();
		if (pct >= 90) return "bg-error";
		if (pct >= 70) return "bg-warning";
		return "bg-success";
	};

	return (
		<div class={`w-full h-2 rounded-full bg-secondary-medium overflow-hidden ${get(props.class)}`}>
			<Show when={get(props.value) > 0 || hasLimit()}>
				<div class={`h-full rounded-full transition-all ${fillColor()}`} style={{ width: `${percent()}%` }} />
			</Show>
		</div>
	);
};

export default UsageBar;
