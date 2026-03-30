import { LogTerminal } from "~/components";
import { useLastWorkspaceId } from "~/hooks/state-hooks";

interface RunnerLogsProps {
	runnerId: string;
}

const RunnerLogs = (props: RunnerLogsProps) => {
	const [workspaceId] = useLastWorkspaceId();
	const baseUrl = import.meta.env.VITE_BASE_URL as string;
	const wsUrl = baseUrl.replace(/^http/, "ws");

	return (
		<LogTerminal
			restUrl={`${baseUrl}/api/workspace/${workspaceId()}/runner/${props.runnerId}/logs`}
			wsUrl={`${wsUrl}/api/workspace/${workspaceId()}/runner/${props.runnerId}/logs/stream`}
		/>
	);
};

export default RunnerLogs;
