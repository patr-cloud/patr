import { Route } from "@solidjs/router";
import CreateDeploymentPage from "~/pages/deployment/create";
import CreateRunnerPage from "~/pages/runner/create";
import ListRunnersPage from "~/pages/runner/list";

export default function WorkspacedRoutes() {
  return (
    <>
      <Route path="/" component={() => <div>Home</div>} />
      <Route path="/deployments">
        <Route path="/new" component={CreateDeploymentPage} />
      </Route>
      <Route path="/runners">
        <Route path="/" component={ListRunnersPage} />
        <Route path="/new" component={CreateRunnerPage} />
      </Route>
    </>
  );
}
