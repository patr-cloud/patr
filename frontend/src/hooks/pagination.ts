import { createEffect, createSignal, untrack, type Accessor } from "solid-js";

export interface PaginationState {
	/** The current 0-indexed page number */
	page: () => number;
	/** The number of items per page */
	count: () => number;
	/** The total number of items across all pages */
	totalCount: () => number;
	/** The total number of pages, derived from totalCount / count */
	totalPages: () => number;
	/** Whether a previous page is available */
	canPrev: () => boolean;
	/** Whether a next page is available */
	canNext: () => boolean;
	/** Set the current page (0-indexed) */
	setPage: (page: number) => void;
	/** Set the number of items per page. Resets to page 0. */
	setCount: (count: number) => void;
	/** Call this with the value of the `x-total-count` response header after each fetch */
	setTotalCount: (total: number) => void;
}

/**
 * Merges `update` into the current search params via `navigate`.
 *
 * TanStack Router's `useNavigate()` is typed against the union of all routes'
 * search params, so a generic `(prev) => ({...prev, ...update})` updater
 * doesn't satisfy any single route's type. We widen the function signature here
 * so callers can pass `navigate` without per-call-site casts.
 */
type NavigateFn = (opts: Record<string, unknown>) => void;

/** The search params a paginated list keeps in its URL. */
export type PaginationSearch = { page?: number; count?: number };

/**
 * The `validateSearch` for paginated list routes. Coerces `page` and `count` to
 * numbers, so they're written as `?page=2` rather than JSON-quoted strings.
 * Older links carried the quoted form (`?page=%222%22`), which parses to the
 * string `"2"`, so strings are coerced too.
 */
export const paginationSearch = (search: Record<string, unknown>): PaginationSearch => {
	const toNumber = (value: unknown, min: number) => {
		const n = typeof value === "string" ? Number(value) : value;
		return typeof n === "number" && Number.isInteger(n) && n >= min ? n : undefined;
	};
	return { page: toNumber(search.page, 0), count: toNumber(search.count, 1) };
};

/**
 * Creates a pagination state object backed by URL search params (`?page=0&count=20`).
 * Page and count survive navigation and can be bookmarked / shared.
 *
 * `page` is 0-indexed internally but stored as-is in the URL.
 *
 * @example
 * ```ts
 * const search = Route.useSearch();
 * const navigate = useNavigate();
 * const pagination = createPaginationState({
 *   defaultCount: 20,
 *   search: () => search(),
 *   navigate,
 * });
 *
 * const [items] = createResource(
 *   () => [workspaceId(), pagination.page(), pagination.count()] as const,
 *   async ([wsId, page, count]) => {
 *     const res = await httpRequest(`...?page=${page}&count=${count}`, { method: "GET" });
 *     if (res.ok) pagination.setTotalCount(Number(res.headers.get("x-total-count") ?? 0));
 *     return res;
 *   }
 * );
 * ```
 */
const createPaginationState = (opts: {
	defaultCount?: number;
	search: Accessor<PaginationSearch>;
	navigate: NavigateFn;
}): PaginationState => {
	// Must match the API's ListResourceQuery::DEFAULT_PAGE_SIZE (20). The list
	// endpoints use that default when no `count` is sent, so if this diverges
	// the page math invents a phantom trailing page whose fetch 400s with
	// pageOutOfBounds.
	const defaultCount = opts.defaultCount ?? 20;
	const [totalCount, setTotalCount] = createSignal(0);

	const page = () => opts.search()?.page ?? 0;

	const count = () => opts.search()?.count ?? defaultCount;

	const totalPages = () => Math.max(1, Math.ceil(totalCount() / count()));
	const canPrev = () => page() > 0;
	const canNext = () => page() < totalPages() - 1;

	const updateSearch = (update: PaginationSearch) => {
		opts.navigate({
			search: (prev: Record<string, unknown>) => ({ ...prev, ...update }),
			replace: true,
		});
	};

	const setPage = (p: number) => {
		const clamped = Math.max(0, Math.min(p, totalPages() - 1));
		updateSearch({ page: clamped });
	};

	const setCount = (c: number) => {
		updateSearch({ count: c, page: 0 });
	};

	return { page, count, totalCount, totalPages, canPrev, canNext, setPage, setCount, setTotalCount };
};

/**
 * When a list query lands on an out-of-bounds page (the API returns
 * `pageOutOfBounds` — e.g. items were deleted while we were on a later page),
 * step back one page instead of surfacing an error. Stepping back re-queries;
 * if that page is also out of bounds it steps back again, converging on the
 * last page that has rows (or page 0, which never reports out-of-bounds).
 *
 * @param isError       the list query's `isError` accessor
 * @param errorMessage  the list query's `error?.message` accessor (the API
 *                      error code thrown by the fetch hook)
 * @param pagination    the pagination state to step back
 */
export const recoverFromOutOfBounds = (
	isError: () => boolean,
	errorMessage: () => string | undefined,
	pagination: PaginationState
): void => {
	createEffect(() => {
		if (isError() && errorMessage() === "pageOutOfBounds" && pagination.page() > 0) {
			// Navigating reads and writes router state, so keep it out of this
			// effect's dependencies or the effect re-runs itself forever.
			untrack(() => pagination.setPage(pagination.page() - 1));
		}
	});
};

export default createPaginationState;
