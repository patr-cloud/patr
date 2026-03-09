import { createSignal } from "solid-js";
import { useSearchParams } from "@solidjs/router";

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
 * Creates a pagination state object backed by URL search params (`?page=0&count=20`).
 * Page and count survive navigation and can be bookmarked / shared.
 *
 * `page` is 0-indexed internally but stored as-is in the URL.
 *
 * @example
 * ```ts
 * const pagination = createPaginationState({ defaultCount: 20 });
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
const createPaginationState = (opts?: { defaultCount?: number }): PaginationState => {
	const defaultCount = opts?.defaultCount ?? 15;
	const [searchParams, setSearchParams] = useSearchParams<{ page?: string; count?: string }>();
	const [totalCount, setTotalCount] = createSignal(0);

	const page = () => {
		const p = parseInt(searchParams.page ?? "0", 10);
		return isNaN(p) || p < 0 ? 0 : p;
	};

	const count = () => {
		const c = parseInt(searchParams.count ?? String(defaultCount), 10);
		return isNaN(c) || c < 1 ? defaultCount : c;
	};

	const totalPages = () => Math.max(1, Math.ceil(totalCount() / count()));
	const canPrev = () => page() > 0;
	const canNext = () => page() < totalPages() - 1;

	const setPage = (p: number) => {
		const clamped = Math.max(0, Math.min(p, totalPages() - 1));
		setSearchParams({ page: String(clamped) }, { replace: true });
	};

	const setCount = (c: number) => {
		setSearchParams({ count: String(c), page: "0" }, { replace: true });
	};

	return { page, count, totalCount, totalPages, canPrev, canNext, setPage, setCount, setTotalCount };
};

export default createPaginationState;
