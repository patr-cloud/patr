import { createReconnectingWS, WSMessage } from "@solid-primitives/websocket";
import { createMemo, createResource } from "solid-js";
import { createStore } from "solid-js/store";
import { GetDeploymentLogsResponse } from "~/bindings";
import { useToast } from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

interface DeploymentLogsProps {
  deploymentId: string;
}

const DeploymentLogs = (props: DeploymentLogsProps) => {
  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();
  const toast = useToast();

  const ws = createReconnectingWS(
    `ws://localhost:3001/api/workspace/${workspaceId()}/deployment/${
      props.deploymentId
    }/logs`
  );
  const [logs, setLogs] = createStore<WSMessage[]>([]);

  ws.addEventListener("message", (event) => {
    const message: WSMessage = JSON.parse(event.data);
    console.log("Received log message:", message);
    setLogs((prevLogs) => [...prevLogs, message]);
  });

  const resourceParamsDeploymentLogs = createMemo(() => {
    return [authState(), workspaceId(), props.deploymentId] as const;
  });

  const [deploymentLogs] = createResource(
    resourceParamsDeploymentLogs,
    async ([auth, wsId, id]) => {
      if (!wsId || !auth || auth.type !== "LoggedIn" || id === "") {
        return undefined;
      }
      const response = await httpRequest<GetDeploymentLogsResponse>(
        `${
          import.meta.env.VITE_BASE_URL
        }/api/workspace/${wsId}/deployment/${id}/logs`,
        {
          method: "GET",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${auth.accessToken}`,
          },
        }
      );

      if (!response.ok) {
        console.error("Failed to fetch deployment logs:", response.data.error);
        toast("Failed to fetch deployment logs", "error");
        return undefined;
      }

      return response.data;
    }
  );

  return <div class="text-white"></div>;
};

export default DeploymentLogs;
