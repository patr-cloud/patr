import { Route } from "@solidjs/router";
import { ParentProps } from "solid-js";
import CreateDeploymentPage from "~/pages/deployment/create";
import DeploymentInfo from "~/pages/deployment/deployment";
import ListDeploymentsPage from "~/pages/deployment/list";
import CreateRunnerPage from "~/pages/runner/create";
import ListRunnersPage from "~/pages/runner/list";

const WorkspacedLayout = (props: ParentProps<{}>) => {
  return <>{props.children}</>;
};

export default function WorkspacedRoutes() {
  return (
    <Route path="/" component={WorkspacedLayout}>
      <Route path="/" component={() => <div>Home</div>} />
      <Route path="/deployments">
        <Route path="/" component={ListDeploymentsPage} />
        <Route path="/new" component={CreateDeploymentPage} />
        <Route path="/:id" component={DeploymentInfo} />
      </Route>
      <Route path="/runners">
        <Route path="/" component={ListRunnersPage} />
        <Route path="/new" component={CreateRunnerPage} />
      </Route>
    </Route>
  );
}
