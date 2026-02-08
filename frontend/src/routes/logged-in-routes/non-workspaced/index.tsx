import { Route } from "@solidjs/router";
import CreateApiTokens from "~/pages/user/api-tokens/create";
import ApiTokenInfo from "~/pages/user/api-tokens/info";
import ListApiTokens from "~/pages/user/api-tokens/list";
import UserSettingsPage from "~/pages/user/settings";
import CreateWorkspace from "~/pages/workspace/create";

export default function NonWorkspacedRoutes() {
	return (
		<>
			<Route path="/workspace">
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
