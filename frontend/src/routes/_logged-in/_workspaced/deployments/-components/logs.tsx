import { useParams } from "@tanstack/solid-router";
import { LogTerminal } from "~/components";
import { useLastWorkspaceId } from "~/hooks/state-hooks";

interface DeploymentLogsProps {
	deploymentId: string;
}

const DeploymentLogs = (props: DeploymentLogsProps) => {
	const [workspaceId] = useLastWorkspaceId();
	const params = useParams({ from: "/_logged-in/_workspaced/deployments/$id" });
	const baseUrl = import.meta.env.VITE_BASE_URL as string;
	const wsUrl = baseUrl.replace(/^http/, "ws");

	return (
		<LogTerminal
			restUrl={`${baseUrl}/api/workspace/${workspaceId()}/deployment/${params().id}/logs`}
			wsUrl={`${wsUrl}/api/workspace/${workspaceId()}/deployment/${params().id}/logs/stream`}
		/>
	);
};

export default DeploymentLogs;
