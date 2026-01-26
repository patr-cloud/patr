import { Navigate, Route } from "@solidjs/router";
import { createEffect, lazy, ParentProps } from "solid-js";
import useFetchWorkspaces from "~/hooks/use-fetch/use-fetch-workspaces";
import useFetchUserPermissions from "~/hooks/use-fetch/use-user-permissions";
import DeploymentInfo from "~/pages/deployment/deployment";

const CreateDeploymentPage = lazy(() => import("~/pages/deployment/create"));
// const DeploymentInfo = lazy(() => import("~/pages/deployment/deployment"));
const ListDeploymentsPage = lazy(() => import("~/pages/deployment/list"));
const DomainInfo = lazy(() => import("~/pages/domain/domain-info"));
const ManagedUrlPage = lazy(() => import("~/pages/managed-url"));
const CreateRunnerPage = lazy(() => import("~/pages/runner/create"));
const ListRunnersPage = lazy(() => import("~/pages/runner/list"));
const ListDomainsPage = lazy(() => import("~/pages/domain/list"));
const CreateDomainPage = lazy(() => import("~/pages/domain/create"));
const ManageWorkspace = lazy(() => import("~/pages/workspace/manage-workspace"));
const ListWorkspaces = lazy(() => import("~/pages/workspace/list"));
const ManageRoles = lazy(() => import("~/pages/workspace/roles/manage-roles"));
const CreateRoles = lazy(() => import("~/pages/workspace/roles/create-roles"));

const WorkspacedLayout = (props: ParentProps<{}>) => {
	const [workspaces] = useFetchWorkspaces();
	useFetchUserPermissions();

	createEffect(() => {
		if (workspaces.state === "ready") {
			console.log("workspaces:", workspaces());
			const workspaceLength = workspaces()?.workspaces?.length || 0;
			if (workspaceLength === 0) {
				return <Navigate href="/onboard" />;
			}
		}
	});

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
			<Route path="/managed-urls">
				<Route path="/" component={ManagedUrlPage} />
			</Route>
			<Route path="/domains">
				<Route path="/" component={ListDomainsPage} />
				<Route path="/new" component={CreateDomainPage} />
				<Route path="/:id" component={DomainInfo} />
			</Route>
			<Route path="/workspaces">
				<Route path="/" component={ListWorkspaces} />
				<Route path="/:id" component={ManageWorkspace} />
				<Route path="/:id/roles" component={ManageRoles} />
				<Route path="/:id/roles/new" component={CreateRoles} />
			</Route>
		</Route>
	);
}
