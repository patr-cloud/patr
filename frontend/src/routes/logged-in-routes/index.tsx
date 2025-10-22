import { Route } from "@solidjs/router";
import WorkspacedRoutes from "./workspaced";
import NonWorkspacedRoutes from "./non-workspaced";
import { createResource, ParentProps } from "solid-js";
import { useAuthState } from "~/utils/state";
import { doFetch } from "~/utils/do-fetch";
import { ListUserWorkspacesResponse } from "~/bindings";

export const PageWrapper = (props: ParentProps<{}>) => {
  const [authState, _] = useAuthState();

  const [workspace] = createResource<ListUserWorkspacesResponse>(async () => {
    const auth = authState();
    const response = await doFetch<ListUserWorkspacesResponse>(
      "http://localhost:3001/api/user/workspaces",
      {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${
            auth.type === "LoggedIn" ? auth.accessToken : ""
          }`,
        },
      }
    );
    return response.data;
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
      <Route path="/" />
      <WorkspacedRoutes />
      <NonWorkspacedRoutes />
    </Route>
  );
}
