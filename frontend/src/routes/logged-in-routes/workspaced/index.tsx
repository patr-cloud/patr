import { Route } from "@solidjs/router";
import CreateDeploymentPage from "~/pages/deployment/create";

export default function WorkspacedRoutes() {
  return (
    <>
      <Route path="/" component={() => <div>Home</div>} />
      <Route path="/deployments">
        <Route path="/new" component={CreateDeploymentPage} />
      </Route>
    </>
  );
}
