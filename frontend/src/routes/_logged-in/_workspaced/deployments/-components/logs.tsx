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
import { GetDeploymentLogsResponse } from "~/bindings";
import { LogLine, useToast } from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { useParams } from "@tanstack/solid-router";

interface DeploymentLogsProps {
	deploymentId: string;
}

const DeploymentLogs = (props: DeploymentLogsProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const params = useParams({ from: "/_logged-in/_workspaced/deployments/$id" });

	const baseUrl = import.meta.env.VITE_BASE_URL as string;
	const wsUrl = baseUrl.replace(/^http/, "ws");
	const ws = createWS(`${wsUrl}/api/workspace/${workspaceId()}/deployment/${props.deploymentId}/logs/stream`);
	const [, setLogs] = createStore<WSMessage[]>([]);

	ws.addEventListener("message", (event) => {
		const message: WSMessage = JSON.parse(event.data);
		setLogs((prevLogs) => [...prevLogs, message]);
	});

	onCleanup(() => ws.close());

	const resourceParams = createMemo(() => {
		return [authState(), workspaceId(), params().id] as const;
	});

	const [deploymentLogs] = createResource(resourceParams, async ([auth, wsId, id]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || id === "") {
			return undefined;
		}
		const response = await httpRequest<GetDeploymentLogsResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment/${id}/logs`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch deployment logs:", response.data.error);
			toast("Failed to fetch deployment logs", "error");
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

	createEffect(() => {
		if (deploymentLogs.latest?.logs.length) {
			queueMicrotask(() => {
				if (isAtBottom()) scrollToBottom();
			});
		}
	});

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
				<span class="text-xxs text-white/30 font-log">{deploymentLogs.latest?.logs.length ?? 0} entries</span>
			</div>

			{/* Log body */}
			<div ref={scrollRef} onScroll={handleScroll} class="flex-1 overflow-auto bg-secondary py-xs">
				<Suspense
					fallback={
						<div class="flex items-center justify-center h-32 text-xs text-grey/50">Loading logs...</div>
					}
				>
					<Show
						when={deploymentLogs.latest && deploymentLogs.latest.logs.length > 0}
						fallback={
							<div class="flex flex-col items-center justify-center h-32 gap-xs">
								<span class="text-xs text-grey/40">No log entries</span>
							</div>
						}
					>
						<For each={deploymentLogs.latest!.logs}>
							{(log, i) => <LogLine log={log} lineNum={i() + 1} />}
						</For>
					</Show>
				</Suspense>
			</div>
		</div>
	);
};

export default DeploymentLogs;
