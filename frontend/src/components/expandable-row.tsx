import { createSignal, JSX, Show } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";

interface ExpandableRowProps {
	/**
	 * Renders the always-visible summary row's cells (the `<td>`s). Receives the
	 * current open state so the caller can draw a chevron that points the right
	 * way.
	 */
	summary: (open: boolean) => JSX.Element;
	/** The panel revealed below the summary row while it is open. */
	children: JSX.Element;
	/**
	 * When true the row is not expandable: it renders the summary with no toggle
	 * and never reveals a panel. `summary` is still called with `false`.
	 */
	disabled?: MaybeAccessor<boolean>;
	/** Start expanded. Defaults to collapsed. */
	defaultOpen?: boolean;
	/**
	 * Called whenever the row is toggled, with the new open state. Handy for
	 * lazily fetching the panel's data the first time it opens.
	 */
	onToggle?: (open: boolean) => void;
	/** Additional classes for the summary `<tr>`. */
	class?: MaybeAccessor<string>;
}

/**
 * A table row that can expand in place to reveal a full-width detail panel
 * beneath it. Built to be returned from a {@link Table}'s `renderRow`, it keeps
 * its own open state (seeded by `defaultOpen`) and notifies `onToggle` on every
 * change. The caller owns the summary cells and the chevron so the row lines up
 * with the table's columns; this component only handles the toggle interaction
 * and the reveal.
 */
const ExpandableRow = (props: ExpandableRowProps) => {
	const [open, setOpen] = createSignal(props.defaultOpen ?? false);

	const toggle = () => {
		if (get(props.disabled)) return;
		const next = !open();
		setOpen(next);
		props.onToggle?.(next);
	};

	return (
		<>
			<tr
				role="row"
				tabIndex={get(props.disabled) ? undefined : 0}
				onClick={toggle}
				onKeyDown={(e) => {
					if (e.key === "Enter" || e.key === " ") {
						e.preventDefault();
						toggle();
					}
				}}
				class={`table-row focus-visible:outline-primary ${
					get(props.disabled) ? "" : "cursor-pointer"
				} ${get(props.class) ?? ""}`}
			>
				{props.summary(open())}
			</tr>
			<Show when={open() && !get(props.disabled)}>
				<tr
					role="row"
					class="w-full flex bg-secondary-dark border border-t-0 border-border-color last-of-type:rounded-b-xs"
				>
					<td role="cell" class="w-full min-w-0 p-md">
						{props.children}
					</td>
				</tr>
			</Show>
		</>
	);
};

export default ExpandableRow;
