import { createSignal, type Accessor } from "solid-js";

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
	search: Accessor<{ page?: string; count?: string }>;
	navigate: NavigateFn;
}): PaginationState => {
	const defaultCount = opts.defaultCount ?? 20;
	const [totalCount, setTotalCount] = createSignal(0);

	const page = () => {
		const p = parseInt(String(opts.search()?.page ?? "0"), 10);
		return isNaN(p) || p < 0 ? 0 : p;
	};

	const count = () => {
		const c = parseInt(String(opts.search()?.count ?? String(defaultCount)), 10);
		return isNaN(c) || c < 1 ? defaultCount : c;
	};

	const totalPages = () => Math.max(1, Math.ceil(totalCount() / count()));
	const canPrev = () => page() > 0;
	const canNext = () => page() < totalPages() - 1;

	const updateSearch = (update: Record<string, string>) => {
		opts.navigate({
			search: (prev: Record<string, unknown>) => ({ ...prev, ...update }),
			replace: true,
		});
	};

	const setPage = (p: number) => {
		const clamped = Math.max(0, Math.min(p, totalPages() - 1));
		updateSearch({ page: String(clamped) });
	};

	const setCount = (c: number) => {
		updateSearch({ count: String(c), page: "0" });
	};

	return { page, count, totalCount, totalPages, canPrev, canNext, setPage, setCount, setTotalCount };
};

export default createPaginationState;
