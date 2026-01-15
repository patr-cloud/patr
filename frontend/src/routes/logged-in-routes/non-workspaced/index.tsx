import { Route } from "@solidjs/router";
import CreateApiTokens from "~/pages/user/api-tokens/create";
import ListApiTokens from "~/pages/user/api-tokens/list";
import UserSettingsPage from "~/pages/user/settings";
import CreateWorkspace from "~/pages/workspace/create";
import ListWorkspaces from "~/pages/workspace/list";

export default function NonWorkspacedRoutes() {
  return (
    <>
      <Route path="/workspaces">
        <Route path="/" component={ListWorkspaces} />
        <Route path="/new" component={CreateWorkspace} />
      </Route>
      <Route path="/profile">
        <Route path="/" component={UserSettingsPage} />
        <Route path="/api-tokens">
          <Route path="/" component={ListApiTokens} />
          <Route path="/new" component={CreateApiTokens} />
        </Route>
      </Route>
    </>
  );
}
