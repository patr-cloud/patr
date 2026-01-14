import { Route, Navigate } from "@solidjs/router";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { createEffect, createResource, onMount, ParentProps } from "solid-js";
import { useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import { ListUserWorkspacesResponse } from "~/bindings";

import WorkspacedRoutes from "./workspaced";
import NonWorkspacedRoutes from "./non-workspaced";
import { useToast } from "~/components";

import Sidebar from "~/components/sidebar";
import TopBar from "~/components/top-bar";
import WorkspaceOnboardPage from "~/pages/workspace/onboard";

export const PageWrapper = (props: ParentProps<{}>) => {
  const [authState, _] = useAuthState();
  const [workspaceId, setWorkspaceId] = useLastWorkspaceId();
  const toast = useToast();

  if (!authState() || authState()?.type === "LoggedOut") {
    console.log("Navigating to /login due to LoggedOut state");
    return <Navigate href="/login" />;
  }

  onMount(() => {
    const auth = authState();
    const currentWorkspace = workspaceId();
    console.log(auth, currentWorkspace);
    if (auth === null) {
      return;
    }
    if (auth.type !== "LoggedIn") {
      return <Navigate href="/login" />;
    }
  });

  const [workspaceResource] = createResource(authState, async (auth) => {
    if (auth === null || auth.type !== "LoggedIn") {
      return { workspaces: [] };
    }
    const response = await httpRequest<ListUserWorkspacesResponse>(
      `${import.meta.env.VITE_BASE_URL}/api/user/workspaces`,
      {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
      }
    );

    if (!response.ok) {
      console.error("Failed to fetch workspaces:", response.data.error);
      toast("Failed to fetch workspaces", "error");
      return { workspaces: [] };
    }

    return response.data;
  });

  createEffect(() => {
    if (!workspaceId()) {
      const workspaces = workspaceResource();
      if (workspaces && workspaces.workspaces.length > 0) {
        setWorkspaceId(workspaces.workspaces[0].id);
      }
    }
  });

  return (
    <main class="bg-secondary w-full min-h-screen h-screen flex">
      <Sidebar />
      <div class="flex-1 flex flex-col overflow-hidden">
        <TopBar />
        <div class="flex-1 overflow-auto">{props.children}</div>
      </div>
    </main>
  );
};

export default function LoggedInRoutes() {
  return (
    <Route path="/">
      <Route path="/" component={PageWrapper}>
        <WorkspacedRoutes />
        <NonWorkspacedRoutes />
      </Route>
      <Route path="/onboard" component={WorkspaceOnboardPage} />
    </Route>
  );
}
