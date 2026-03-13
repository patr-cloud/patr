import { createWS, WSMessage } from "@solid-primitives/websocket";
import { createMemo, createResource, For, onCleanup, Show, Suspense } from "solid-js";
import { createStore } from "solid-js/store";
import { GetDeploymentLogsResponse } from "~/bindings";
import { useToast } from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import LogStatement from "../log-statement";
import { useParams } from "@solidjs/router";

interface DeploymentLogsProps {
	deploymentId: string;
}

const DeploymentLogs = (props: DeploymentLogsProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const params = useParams();

	const baseUrl = import.meta.env.VITE_BASE_URL as string;
	const wsUrl = baseUrl.replace(/^http/, "ws");
	const ws = createWS(
		`${wsUrl}/api/workspace/${workspaceId()}/deployment/${props.deploymentId}/logs/stream`
	);
	const [, setLogs] = createStore<WSMessage[]>([]);

	ws.addEventListener("message", (event) => {
		const message: WSMessage = JSON.parse(event.data);
		console.log("Received log message:", message);
		setLogs((prevLogs) => [...prevLogs, message]);
	});

	onCleanup(() => ws.close());

	const resourceParamsDeploymentLogs = createMemo(() => {
		return [authState(), workspaceId(), params.id] as const;
	});

	const [deploymentLogs] = createResource(resourceParamsDeploymentLogs, async ([auth, wsId, id]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || id === "") {
			console.log("Invalid parameters for fetching deployment logs", wsId, auth, id);
			return undefined;
		}
		const response = await httpRequest<GetDeploymentLogsResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment/${id}/logs`,
			{
				method: "GET",
				headers: {
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch deployment logs:", response.data.error);
			toast("Failed to fetch deployment logs", "error");
			return undefined;
		}

		console.log("Fetched deployment logs:", response.data);

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
							<Show when={deploymentLogs.latest}>
								<For each={deploymentLogs.latest!.logs}>{(log) => <LogStatement log={log} />}</For>
							</Show>
						</div>
					</Suspense>
				</div>
			</div>
		</div>
	);
};

export default DeploymentLogs;
