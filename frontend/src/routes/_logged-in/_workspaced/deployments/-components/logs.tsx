import { LogTerminal } from "~/components";
import { useLastWorkspaceId } from "~/hooks/state-hooks";

interface DeploymentLogsProps {
	deploymentId: string;
}

const DeploymentLogs = (props: DeploymentLogsProps) => {
	const [workspaceId] = useLastWorkspaceId();
	const deploymentId = () => props.deploymentId;
	const baseUrl = import.meta.env.VITE_BASE_URL as string;
	const wsUrl = baseUrl.replace(/^http/, "ws");

	return (
		<LogTerminal
			restUrl={`${baseUrl}/api/workspace/${workspaceId()}/deployment/${deploymentId()}/logs`}
			wsUrl={`${wsUrl}/api/workspace/${workspaceId()}/deployment/${deploymentId()}/logs/stream`}
		/>
	);
};

export default DeploymentLogs;
