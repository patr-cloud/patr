import { createWS, WSMessage } from "@solid-primitives/websocket";
import { createEffect, createMemo, createResource, createSignal, For, on, onCleanup, onMount, Show } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { InputDropdown, useToast } from "~/components";
import LogLine from "~/components/log-line";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

interface LogTerminalProps {
	/** REST endpoint for fetching historical logs (without query params) */
	restUrl: string;
	/** WebSocket endpoint for live tail */
	wsUrl: string;
}

interface LogEntry {
	timestamp: Date | string;
	log: string;
}

interface LogsResponse {
	logs: LogEntry[];
}

const TIME_RANGES = [
	{ label: "Last 15 min", value: "900" },
	{ label: "Last 1 hour", value: "3600" },
	{ label: "Last 6 hours", value: "21600" },
	{ label: "Last 12 hours", value: "43200" },
	{ label: "Last 24 hours", value: "86400" },
	{ label: "Last 7 days", value: "604800" },
];

/** Debounce a value by `ms` milliseconds */
const useDebounce = (value: () => string, ms: number) => {
	const [debounced, setDebounced] = createSignal(value());
	let timer: ReturnType<typeof setTimeout>;
	createEffect(() => {
		const v = value();
		clearTimeout(timer);
		timer = setTimeout(() => setDebounced(v), ms);
	});
	onCleanup(() => clearTimeout(timer));
	return debounced;
};

const LogTerminal = (props: LogTerminalProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	// --- State ---
	const [searchInput, setSearchInput] = createSignal("");
	const debouncedSearch = useDebounce(searchInput, 300);
	const [timeRange, setTimeRange] = createSignal("3600");
	const [logs, setLogs] = createStore<LogEntry[]>([]);
	const [isLoadingMore, setIsLoadingMore] = createSignal(false);
	const [hasMoreLogs, setHasMoreLogs] = createSignal(true);
	const isSearching = () => debouncedSearch().length > 0;

	// --- REST fetch ---
	const fetchParams = createMemo(() => {
		return [authState(), workspaceId(), props.restUrl, debouncedSearch(), timeRange()] as const;
	});

	const [initialLogs] = createResource(fetchParams, async ([auth, wsId, restUrl, search, _range]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") return undefined;

		const params = new URLSearchParams();
		params.set("limit", "100");
		if (search) params.set("search", search);

		const response = await httpRequest<LogsResponse>(`${restUrl}?${params.toString()}`, {
			method: "GET",
		});

		if (!response.ok) {
			toast("Failed to fetch logs", "error");
			return undefined;
		}

		return response.data;
	});

	// When REST data arrives, reset the log store
	createEffect(
		on(
			() => initialLogs(),
			(data) => {
				if (data?.logs) {
					setLogs(data.logs);
					setHasMoreLogs(data.logs.length >= 100);
					// Scroll to bottom after render
					queueMicrotask(scrollToBottom);
				} else {
					setLogs([]);
					setHasMoreLogs(false);
				}
			}
		)
	);

	// --- WebSocket ---
	createEffect(() => {
		const ws = createWS(props.wsUrl);

		ws.addEventListener("message", (event) => {
			try {
				const message: WSMessage = JSON.parse(event.data);
				const entries = Array.isArray(message) ? message : [message];

				for (const entry of entries) {
					const logEntry = entry as LogEntry;
					// Client-side search filter for WS messages
					if (isSearching() && !logEntry.log.toLowerCase().includes(debouncedSearch().toLowerCase())) {
						continue;
					}
					setLogs(
						produce((prev) => {
							prev.push(logEntry);
						})
					);
				}

				// Auto-scroll if at bottom
				if (isAtBottom()) {
					queueMicrotask(scrollToBottom);
				}
			} catch {
				// ignore malformed messages
			}
		});

		onCleanup(() => ws.close());
	});

	// --- Load more ---
	const loadMore = async () => {
		if (isLoadingMore() || logs.length === 0) return;
		setIsLoadingMore(true);

		const oldestLog = logs[0];
		const params = new URLSearchParams();
		params.set("limit", "100");
		if (typeof oldestLog.timestamp === "string") {
			params.set("end_time", oldestLog.timestamp);
		} else {
			params.set("end_time", oldestLog.timestamp.toISOString());
		}
		if (debouncedSearch()) {
			params.set("search", debouncedSearch());
		}

		const response = await httpRequest<LogsResponse>(`${props.restUrl}?${params.toString()}`, {
			method: "GET",
		});

		setIsLoadingMore(false);

		if (!response.ok) {
			toast("Failed to load more logs", "error");
			return;
		}

		const olderLogs = response.data.logs;
		if (olderLogs.length < 100) {
			setHasMoreLogs(false);
		}

		if (olderLogs.length > 0) {
			// Preserve scroll position
			const scrollEl = scrollRef;
			const prevHeight = scrollEl?.scrollHeight ?? 0;
			const prevTop = scrollEl?.scrollTop ?? 0;

			setLogs(
				produce((prev) => {
					prev.unshift(...olderLogs);
				})
			);

			// Restore scroll position after prepend
			queueMicrotask(() => {
				if (scrollEl) {
					const newHeight = scrollEl.scrollHeight;
					scrollEl.scrollTop = prevTop + (newHeight - prevHeight);
				}
			});
		}
	};

	// --- Auto-scroll ---
	let scrollRef!: HTMLDivElement;
	const [isAtBottom, setIsAtBottom] = createSignal(true);

	const scrollToBottom = () => {
		if (scrollRef) scrollRef.scrollTop = scrollRef.scrollHeight;
	};

	const handleScroll = () => {
		if (!scrollRef) return;
		const atBottom = scrollRef.scrollHeight - scrollRef.scrollTop - scrollRef.clientHeight < 30;
		setIsAtBottom(atBottom);
	};

	onMount(() => queueMicrotask(scrollToBottom));

	// --- Derived ---
	const entryCount = () => logs.length;

	return (
		<div
			class="w-full flex flex-col overflow-hidden rounded-xs border border-border-color"
			style={{ height: "calc(100vh - 250px)" }}
		>
			{/* Header bar */}
			<div class="flex items-center gap-sm px-md py-xs bg-secondary-light border-b border-border-color shrink-0">
				{/* Status indicator */}
				<div class="flex items-center gap-xxs shrink-0">
					<Show
						when={!isSearching()}
						fallback={
							<>
								<span class="w-2 h-2 rounded-full bg-info" />
								<span class="text-xxs text-info font-medium">SEARCH</span>
							</>
						}
					>
						<span class="w-2 h-2 rounded-full bg-success animate-pulse" />
						<span class="text-xxs text-grey font-medium">LIVE</span>
					</Show>
				</div>

				{/* Search input */}
				<div class="flex-1 relative">
					<input
						type="text"
						value={searchInput()}
						onInput={(e) => setSearchInput(e.currentTarget.value)}
						placeholder="Search logs..."
						class="w-full bg-secondary-medium border border-border-color rounded-xs px-sm py-1 text-xs font-log text-white/80 placeholder:text-white/40 focus:outline-none focus:border-primary/50"
					/>
					<Show when={searchInput().length > 0}>
						<button
							onClick={() => setSearchInput("")}
							class="absolute right-1.5 top-1/2 -translate-y-1/2 text-white/30 hover:text-white/60 text-xs leading-none"
						>
							&times;
						</button>
					</Show>
				</div>

				{/* Time range */}
				<div class="w-32 shrink-0">
					<InputDropdown
						options={TIME_RANGES}
						value={timeRange()}
						onSelect={(val) => setTimeRange(val)}
						placeholder="Time range"
						styleVariant="dark"
					/>
				</div>

				{/* Entry count */}
				<span class="text-xxs text-white/30 font-log shrink-0 tabular-nums">{entryCount()} entries</span>
			</div>

			{/* Log body */}
			<div ref={scrollRef} onScroll={handleScroll} class="flex-1 overflow-auto bg-secondary py-xs">
				{/* Load more button */}
				<Show when={hasMoreLogs() && logs.length > 0}>
					<div class="flex justify-center py-xs">
						<button
							onClick={loadMore}
							disabled={isLoadingMore()}
							class="text-xxs font-log text-primary/60 hover:text-primary px-sm py-xxs rounded-xs border border-border-color hover:border-primary/30 bg-secondary-light transition-colors disabled:opacity-50"
						>
							{isLoadingMore() ? "Loading..." : "Load more entries"}
						</button>
					</div>
				</Show>

				{/* Log lines */}
				<Show
					when={logs.length > 0}
					fallback={
						<div class="flex flex-col items-center justify-center h-32 gap-xs">
							<Show
								when={isSearching()}
								fallback={<span class="text-xs text-grey/40">No log entries</span>}
							>
								<span class="text-xs text-grey/40">
									No logs matching &lsquo;{debouncedSearch()}&rsquo;
								</span>
								<span class="text-xxs text-grey/25">Try increasing the time range</span>
							</Show>
						</div>
					}
				>
					<For each={logs}>{(log, i) => <LogLine log={log} lineNum={i() + 1} />}</For>
				</Show>
			</div>
		</div>
	);
};

export default LogTerminal;
