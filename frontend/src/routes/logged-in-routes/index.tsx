import { Route, Navigate } from "@solidjs/router";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { createEffect, createResource, ParentProps } from "solid-js";
import { useAuthState } from "~/hooks";
import { doFetch } from "~/utils/do-fetch";
import { ListUserWorkspacesResponse } from "~/bindings";

import WorkspacedRoutes from "./workspaced";
import NonWorkspacedRoutes from "./non-workspaced";
import Home from "~/pages/home";

export const PageWrapper = (props: ParentProps<{}>) => {
  const [authState, _] = useAuthState();
  const [workspaceId, setWorkspaceId] = useLastWorkspaceId();

  console.log(
    "Rendering PageWrapper with authState:",
    authState(),
    authState()?.type === "LoggedOut"
  );

  if (!authState() || authState()?.type === "LoggedOut") {
    console.log("Navigating to /login due to LoggedOut state");
    return <Navigate href="/login" />;
  }

  createEffect(() => {
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
    const response = await doFetch<ListUserWorkspacesResponse>(
      `${import.meta.env.VITE_BASE_URL}/api/user/workspaces`,
      {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
      }
    );

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
      <aside class="bg-secondary w-64 h-screen shadow-xl/30"></aside>
      <div class="flex-1">{props.children}</div>
    </main>
  );
};

export default function LoggedInRoutes() {
  return (
    <Route path="/" component={PageWrapper}>
      <Route path="/" component={Home} />
      <WorkspacedRoutes />
      <NonWorkspacedRoutes />
    </Route>
  );
}
