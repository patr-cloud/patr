import { createSignal, For, mergeProps, Show } from "solid-js";
import { get } from "~/utils/func";
import { MaybeAccessor } from "~/utils/types";
import type { PaginationState } from "~/hooks/pagination";

interface PaginationProps {
	/** The pagination state object returned by createPaginationState() */
	state: PaginationState;
	/** Whether to show the page-size selector. Defaults to true. */
	showPageSizeSelector?: boolean;
	/** Page size options. Defaults to [10, 14, 20, 50, 100]. */
	countOptions?: number[];
	/** Whether to show the "Go to page" input. Defaults to true. */
	showGoToPage?: boolean;
	/** Whether the list is currently loading — disables all controls. */
	loading?: MaybeAccessor<boolean>;
	/** Additional classes for the root element */
	class?: MaybeAccessor<string>;
}

/**
 * Builds a compact page-window array with at most 7 entries.
 * e.g. for totalPages=20, currentPage=10:  [1, "...", 9, 10, 11, "...", 20]
 */
const buildPageWindow = (current: number, total: number): (number | "...")[] => {
	if (total <= 7) {
		return Array.from({ length: total }, (_, i) => i + 1);
	}

	const first = 1;
	const last = total;
	const cur = current + 1; // convert to 1-indexed for display

	const showLeftEllipsis = cur > 4;
	const showRightEllipsis = cur < total - 3;

	if (!showLeftEllipsis && showRightEllipsis) {
		// near the start: [1 2 3 4 5 ... last]
		return [1, 2, 3, 4, 5, "...", last];
	}

	if (showLeftEllipsis && !showRightEllipsis) {
		// near the end: [1 ... last-4 last-3 last-2 last-1 last]
		return [first, "...", total - 4, total - 3, total - 2, total - 1, last];
	}

	// middle: [1 ... cur-1 cur cur+1 ... last]
	return [first, "...", cur - 1, cur, cur + 1, "...", last];
};

const Pagination = (rawProps: PaginationProps) => {
	const props = mergeProps(
		{
			showPageSizeSelector: true,
			showGoToPage: true,
			countOptions: [10, 14, 20, 50, 100],
		},
		rawProps
	);

	const [jumpValue, setJumpValue] = createSignal("");

	const isLoading = () => get(props.loading) ?? false;

	const handleJump = () => {
		const n = parseInt(jumpValue(), 10);
		if (!isNaN(n)) {
			props.state.setPage(n - 1); // convert to 0-indexed
		}
		setJumpValue("");
	};

	const pageWindow = () => buildPageWindow(props.state.page(), props.state.totalPages());

	const rangeStart = () => props.state.page() * props.state.count() + 1;
	const rangeEnd = () => Math.min((props.state.page() + 1) * props.state.count(), props.state.totalCount());

	const btnBase =
		"h-8 flex items-center justify-center rounded-xs text-sm font-medium transition-colors duration-150 disabled:opacity-40 disabled:cursor-not-allowed";
	const btnPage = `${btnBase} w-8`;
	const btnInactive = `${btnPage} bg-secondary-light text-grey enabled:hover:bg-secondary-medium enabled:hover:text-white`;
	const btnActive = `${btnPage} bg-primary text-secondary font-semibold`;
	const btnFirstLast = `${btnBase} px-sm gap-xxs bg-secondary-light text-white enabled:hover:bg-secondary-medium`;

	return (
		<div
			class={`flex flex-col gap-sm sm:flex-row sm:items-center sm:justify-between mt-md ${get(props.class) ?? ""}`}
		>
			{/* Left: item range label */}
			<p class="text-sm text-grey flex-1">
				<Show when={props.state.totalCount() > 0} fallback={<span>No results</span>}>
					Showing{" "}
					<span class="text-white">
						{rangeStart()}–{rangeEnd()}
					</span>{" "}
					of <span class="text-white">{props.state.totalCount()}</span>
				</Show>
			</p>

			{/* Centre: page buttons — hidden (but space preserved) when only one page */}
			<Show when={props.state.totalPages() > 1} fallback={<div class="h-8" />}>
				<div class="flex items-center gap-xs">
					{/* First page */}
					<button
						type="button"
						class={btnFirstLast}
						disabled={!props.state.canPrev() || isLoading()}
						onClick={() => props.state.setPage(0)}
						aria-label="First page"
					>
						<span aria-hidden="true">«</span>First
					</button>

					<For each={pageWindow()}>
						{(entry) => (
							<Show
								when={entry !== "..."}
								fallback={
									<span class="w-8 h-8 flex items-center justify-center text-grey text-sm select-none">
										…
									</span>
								}
							>
								<button
									class={(entry as number) - 1 === props.state.page() ? btnActive : btnInactive}
									disabled={isLoading()}
									onClick={() => props.state.setPage((entry as number) - 1)}
									aria-label={`Page ${entry}`}
									aria-current={(entry as number) - 1 === props.state.page() ? "page" : undefined}
								>
									{entry as number}
								</button>
							</Show>
						)}
					</For>

					{/* Last page */}
					<button
						type="button"
						class={btnFirstLast}
						disabled={!props.state.canNext() || isLoading()}
						onClick={() => props.state.setPage(props.state.totalPages() - 1)}
						aria-label="Last page"
					>
						Last<span aria-hidden="true">»</span>
					</button>
				</div>
			</Show>

			{/* Right: jump-to-page + page-size selector — also hidden when single page */}
			<Show when={props.state.totalPages() > 1} fallback={<div class="flex-1" />}>
				<div class="flex items-center justify-end gap-sm flex-1">
					{/* Jump to page */}
					<Show when={props.showGoToPage}>
						<div class="flex items-center gap-xs">
							<span class="text-sm text-grey whitespace-nowrap">Go to</span>
							<input
								type="number"
								min={1}
								max={props.state.totalPages()}
								disabled={isLoading()}
								value={jumpValue()}
								onInput={(e) => setJumpValue(e.currentTarget.value)}
								onKeyDown={(e) => e.key === "Enter" && handleJump()}
								onBlur={handleJump}
								class="w-14 h-8 px-xs text-sm bg-secondary-light border border-border-color rounded-xs text-white text-center
                               focus:outline-none focus:border-primary disabled:opacity-40 disabled:cursor-not-allowed
                               [appearance:textfield] [&::-webkit-inner-spin-button]:appearance-none [&::-webkit-outer-spin-button]:appearance-none"
								placeholder={String(props.state.page() + 1)}
							/>
						</div>
					</Show>

					{/* Page size selector */}
					<Show when={props.showPageSizeSelector}>
						<select
							disabled={isLoading()}
							value={props.state.count()}
							onChange={(e) => props.state.setCount(Number(e.currentTarget.value))}
							class="h-8 p-xs py-0 text-sm bg-secondary-light border border-border-color rounded-xs text-grey
              focus:outline-none focus:border-primary disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer"
							aria-label="Items per page"
						>
							<For each={props.countOptions}>{(opt) => <option value={opt}>{opt} / page</option>}</For>
						</select>
					</Show>
				</div>
			</Show>
		</div>
	);
};

export default Pagination;
