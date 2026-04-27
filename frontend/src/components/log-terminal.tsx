import { createWS } from "@solid-primitives/websocket";
import { createEffect, createSignal, For, on, onCleanup, onMount, Show } from "solid-js";
import { createStore, produce } from "solid-js/store";
import { createQuery } from "@tanstack/solid-query";
import { useToast } from "~/components";
import LogLine from "~/components/log-line";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { logKeys } from "~/hooks/query-keys";
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
	const [logs, setLogs] = createStore<LogEntry[]>([]);
	const [isLoadingMore, setIsLoadingMore] = createSignal(false);
	const [hasMoreLogs, setHasMoreLogs] = createSignal(true);
	const [isConnected, setIsConnected] = createSignal(false);
	const isSearching = () => debouncedSearch().length > 0;

	// --- REST fetch ---
	const initialLogsQuery = createQuery<LogsResponse>(() => {
		const auth = authState();
		const wsId = workspaceId();
		const search = debouncedSearch();
		return {
			queryKey: logKeys.initial(wsId ?? "", props.restUrl, search),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn",
			meta: { errorMessage: "Failed to fetch logs" },
			queryFn: async () => {
				const params = new URLSearchParams();
				params.set("limit", "100");
				if (search) params.set("search", search);

				const response = await httpRequest<LogsResponse>(`${props.restUrl}?${params.toString()}`, {
					method: "GET",
				});

				if (!response.ok) {
					throw new Error(response.data.error);
				}

				return response.data;
			},
		};
	});

	// When REST data arrives, reset the log store
	createEffect(
		on(
			() => initialLogsQuery.data,
			(data) => {
				if (data?.logs) {
					setLogs(data.logs);
					setHasMoreLogs(data.logs.length >= 100);
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
		let ws: WebSocket;
		let reconnectTimer: ReturnType<typeof setTimeout>;
		let disposed = false;

		const connect = () => {
			ws = createWS(props.wsUrl);

			ws.addEventListener("open", () => setIsConnected(true));
			ws.addEventListener("close", () => {
				setIsConnected(false);
				if (!disposed) {
					reconnectTimer = setTimeout(connect, 500);
				}
			});

			ws.addEventListener("message", (event) => {
				try {
					const message = JSON.parse(event.data);
					if (message?.type !== "LogData") return;
					const entries: LogEntry[] = message.logs ?? [];

					for (const logEntry of entries) {
						if (isSearching() && !logEntry.log.toLowerCase().includes(debouncedSearch().toLowerCase())) {
							continue;
						}
						setLogs(
							produce((prev) => {
								prev.push(logEntry);
							})
						);
					}

					if (isAtBottom()) {
						queueMicrotask(scrollToBottom);
					}
				} catch {
					// ignore malformed messages
				}
			});
		};

		connect();

		onCleanup(() => {
			disposed = true;
			clearTimeout(reconnectTimer);
			ws.close();
			setIsConnected(false);
		});
	});

	// --- Load more (auto-fetch on scroll to top) ---
	const loadMore = async () => {
		if (isLoadingMore() || !hasMoreLogs() || logs.length === 0) return;
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

		// Loki's `end` boundary can be inclusive and ISO-8601 round-tripping can
		// shave sub-millisecond precision, so the API may return entries we
		// already have. Dedupe against the existing oldest slice before
		// prepending — if nothing is new, stop paging.
		const tsKey = (t: Date | string) => (typeof t === "string" ? t : t.toISOString());
		const existingKeys = new Set<string>();
		for (let i = 0; i < Math.min(logs.length, 200); i++) {
			existingKeys.add(`${tsKey(logs[i].timestamp)}|${logs[i].log}`);
		}
		const newOlderLogs = olderLogs.filter((entry) => !existingKeys.has(`${tsKey(entry.timestamp)}|${entry.log}`));

		if (newOlderLogs.length === 0) {
			setHasMoreLogs(false);
			return;
		}

		const scrollEl = scrollRef;
		const prevHeight = scrollEl?.scrollHeight ?? 0;
		const prevTop = scrollEl?.scrollTop ?? 0;

		setLogs(
			produce((prev) => {
				prev.unshift(...newOlderLogs);
			})
		);

		// Restore scroll position synchronously — SolidJS updates the DOM
		// immediately after the store change, so scrollHeight is already correct
		if (scrollEl) {
			scrollEl.scrollTop = prevTop + (scrollEl.scrollHeight - prevHeight);
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

		// Auto-fetch when scrolled to top
		if (scrollRef.scrollTop < 10 && hasMoreLogs() && !isLoadingMore()) {
			loadMore();
		}
	};

	onMount(() => queueMicrotask(scrollToBottom));

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
						<Show
							when={isConnected()}
							fallback={
								<>
									<span class="w-2 h-2 rounded-full bg-warning" />
									<span class="text-xxs text-warning font-medium">CONNECTING</span>
								</>
							}
						>
							<span class="w-2 h-2 rounded-full bg-success animate-pulse" />
							<span class="text-xxs text-grey font-medium">LIVE</span>
						</Show>
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

				{/* Entry count */}
				<span class="text-xxs text-white/30 font-log shrink-0 tabular-nums">{entryCount()} entries</span>
			</div>

			{/* Log body */}
			<div ref={scrollRef} onScroll={handleScroll} class="flex-1 overflow-auto bg-secondary py-xs">
				{/* Top sentinel — spinner or end-of-logs */}
				<Show when={logs.length > 0}>
					<div class="flex items-center justify-center py-sm">
						<Show
							when={hasMoreLogs()}
							fallback={
								<div class="flex items-center gap-xs">
									<span class="w-8 h-px bg-border-color" />
									<span class="text-xxs font-log text-white/20">Beginning of logs</span>
									<span class="w-8 h-px bg-border-color" />
								</div>
							}
						>
							<Show when={isLoadingMore()}>
								<div class="w-3.5 h-3.5 border-2 border-primary/30 border-t-primary rounded-full animate-spin" />
							</Show>
						</Show>
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
