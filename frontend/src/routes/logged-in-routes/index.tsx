import { Route } from "@solidjs/router";
import WorkspacedRoutes from "./workspaced";
import NonWorkspacedRoutes from "./non-workspaced";
import { ParentProps } from "solid-js";

export const PageWrapper = (props: ParentProps<{}>) => {
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
