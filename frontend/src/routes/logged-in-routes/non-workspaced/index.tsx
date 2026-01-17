import { Route } from "@solidjs/router";
import { lazy } from "solid-js";

const CreateApiTokens = lazy(() => import("~/pages/user/api-tokens/create"));
const ApiTokenInfo = lazy(() => import("~/pages/user/api-tokens/info"));
const ListApiTokens = lazy(() => import("~/pages/user/api-tokens/list"));
const UserSettingsPage = lazy(() => import("~/pages/user/settings"));
const CreateWorkspace = lazy(() => import("~/pages/workspace/create"));
const ListWorkspaces = lazy(() => import("~/pages/workspace/list"));

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
					<Route path="/:id" component={ApiTokenInfo} />
				</Route>
			</Route>
		</>
	);
}
