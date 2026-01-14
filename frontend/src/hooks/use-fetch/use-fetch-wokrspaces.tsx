import { createMemo, createResource } from "solid-js";
import { useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { ListUserWorkspacesResponse } from "~/bindings";

const useFetchWorkspaces = () => {
  const [authState] = useAuthState();
  const [workspaceId] = useLastWorkspaceId();
  const toast = useToast();

  const fetchParams = createMemo(() => {
    return [authState(), workspaceId] as const;
  });

  const resource = createResource(fetchParams, async ([auth, wsId]) => {
    if (!wsId || !auth || auth.type !== "LoggedIn") {
      return { workspaces: [] };
    }

    try {
      const response = await httpRequest<ListUserWorkspacesResponse>(
        `${import.meta.env.VITE_BASE_URL}/api/user/workspaces`,
        {
          method: "GET",
        }
      );

      if (!response.ok) {
        console.error("Failed to fetch workspaces:", response.data.error);
        toast("Failed to fetch workspaces", "error");
        return { workspaces: [] };
      }

      return response.data;
    } catch (error) {
      console.error("Error fetching workspaces:", error);
      toast("Failed to load workspaces", "error");
      return { workspaces: [] };
    }
  });

  return resource;
};

export default useFetchWorkspaces;
