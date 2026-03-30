import { createWS, WSMessage } from "@solid-primitives/websocket";
import {
	createEffect,
	createMemo,
	createResource,
	createSignal,
	For,
	onCleanup,
	onMount,
	Show,
	Suspense,
} from "solid-js";
import { createStore } from "solid-js/store";
import { GetRunnerLogsResponse } from "~/bindings";
import { LogLine, useToast } from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

interface RunnerLogsProps {
	runnerId: string;
}

const RunnerLogs = (props: RunnerLogsProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const baseUrl = import.meta.env.VITE_BASE_URL as string;
	const wsUrl = baseUrl.replace(/^http/, "ws");
	const ws = createWS(`${wsUrl}/api/workspace/${workspaceId()}/runner/${props.runnerId}/logs/stream`);
	const [, setLogs] = createStore<WSMessage[]>([]);

	ws.addEventListener("message", (event) => {
		const message: WSMessage = JSON.parse(event.data);
		setLogs((prevLogs) => [...prevLogs, message]);
	});

	onCleanup(() => ws.close());

	const resourceParams = createMemo(() => {
		return [authState(), workspaceId(), props.runnerId] as const;
	});

	const [runnerLogs] = createResource(resourceParams, async ([auth, wsId, runnerId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || !runnerId) {
			return undefined;
		}
		const response = await httpRequest<GetRunnerLogsResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/${runnerId}/logs`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch runner logs:", response.data.error);
			toast("Failed to fetch runner logs", "error");
			return undefined;
		}

		return { logs: response.data.logs };
	});

	// Auto-scroll: stick to bottom like a terminal
	let scrollRef!: HTMLDivElement;
	const [isAtBottom, setIsAtBottom] = createSignal(true);

	const scrollToBottom = () => {
		if (scrollRef) {
			scrollRef.scrollTop = scrollRef.scrollHeight;
		}
	};

	const handleScroll = () => {
		if (!scrollRef) return;
		const threshold = 30;
		const atBottom = scrollRef.scrollHeight - scrollRef.scrollTop - scrollRef.clientHeight < threshold;
		setIsAtBottom(atBottom);
	};

	// Scroll to bottom on initial load
	createEffect(() => {
		if (runnerLogs.latest?.logs.length) {
			// Wait for DOM to render
			queueMicrotask(() => {
				if (isAtBottom()) scrollToBottom();
			});
		}
	});

	// Scroll to bottom on initial mount
	onMount(() => queueMicrotask(scrollToBottom));

	return (
		<div
			class="w-full flex flex-col overflow-hidden rounded-xs border border-border-color"
			style={{ height: "calc(100vh - 250px)" }}
		>
			{/* Header bar */}
			<div class="flex items-center justify-between px-md py-xs bg-secondary-light border-b border-border-color shrink-0">
				<div class="flex items-center gap-xs">
					<span class="w-2 h-2 rounded-full bg-success animate-pulse" />
					<span class="text-xxs text-grey font-medium">LIVE</span>
				</div>
				<span class="text-xxs text-white/30 font-log">{runnerLogs.latest?.logs.length ?? 0} entries</span>
			</div>

			{/* Log body */}
			<div ref={scrollRef} onScroll={handleScroll} class="flex-1 overflow-auto bg-secondary py-xs">
				<Suspense
					fallback={
						<div class="flex items-center justify-center h-32 text-xs text-grey/50">Loading logs...</div>
					}
				>
					<Show
						when={runnerLogs.latest && runnerLogs.latest.logs.length > 0}
						fallback={
							<div class="flex flex-col items-center justify-center h-32 gap-xs">
								<span class="text-xs text-grey/40">No log entries</span>
							</div>
						}
					>
						<For each={runnerLogs.latest!.logs}>{(log, i) => <LogLine log={log} lineNum={i() + 1} />}</For>
					</Show>
				</Suspense>
			</div>
		</div>
	);
};

export default RunnerLogs;
