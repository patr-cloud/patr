import { createWS, WSMessage } from "@solid-primitives/websocket";
import { createMemo, createResource, For, onCleanup, Show, Suspense } from "solid-js";
import { createStore } from "solid-js/store";
import { GetRunnerLogsResponse } from "~/bindings";
import { useToast } from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { FiChevronRight } from "solid-icons/fi";

interface RunnerLogsProps {
	runnerId: string;
}

interface LogEntry {
	timestamp: Date | string;
	log: string;
}

const LogStatement = (props: { log: LogEntry }) => {
	const ts =
		props.log.timestamp instanceof Date
			? props.log.timestamp.toLocaleString()
			: new Date(props.log.timestamp).toLocaleString();

	return (
		<div class="text-grey log-statement flex justify-start items-center w-full hover:bg-grey/60">
			<FiChevronRight class="text-xs text-grey" />
			<time class="text-xxs pr-xs font-log">{ts}</time>-<span class="px-xs font-log">{props.log.log}</span>
		</div>
	);
};

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

		return {
			logs: [
				...response.data.logs,
				{
					timestamp: new Date(),
					log: "Connected to live logs...",
				},
			],
		};
	});

	return (
		<div class="w-full h-full flex flex-col grow overflow-hidden">
			<div class="w-full h-full flex flex-col grow items-start justify-start">
				<div class="w-full h-full br-sm bg-secondary px-xl py-md flex grow flex-col items-start justify-start overflow-auto">
					<Suspense fallback={<div>Loading logs...</div>}>
						<div class="w-full flex flex-col gap-2">
							<Show when={runnerLogs.latest}>
								<For each={runnerLogs.latest!.logs}>{(log) => <LogStatement log={log} />}</For>
							</Show>
						</div>
					</Suspense>
				</div>
			</div>
		</div>
	);
};

export default RunnerLogs;
