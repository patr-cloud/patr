import { Route, useNavigate } from "@solidjs/router";
import { createEffect, ParentProps } from "solid-js";
import HomePage from "~/pages/home";
import CreateDeploymentPage from "~/pages/deployment/create";
import DeploymentInfo from "~/pages/deployment/deployment";
import ListDeploymentsPage from "~/pages/deployment/list";
import DomainInfo from "~/pages/domain/domain-info";
import CreateRunnerPage from "~/pages/runner/create";
import ListRunnersPage from "~/pages/runner/list";
import ListDomainsPage from "~/pages/domain/list";
import CreateDomainPage from "~/pages/domain/create";
import ManageWorkspace from "~/pages/workspace/manage-workspace";
import ManageRoles from "~/pages/workspace/roles/manage-roles";
import CreateRoles from "~/pages/workspace/roles/create-roles";
import RoleInfo from "~/pages/workspace/roles/role-info";
import CreateContainerRepository from "~/pages/container-repository/create";
import ContainerRepositoryInfo from "~/pages/container-repository/container";
import ListContainerRepositories from "~/pages/container-repository/list";
import General from "~/pages/workspace/general";
import { useFetchWorkspaces } from "~/hooks/fetch";
import { useFetchUserPermissions } from "~/hooks/fetch";

const WorkspacedLayout = (props: ParentProps<{}>) => {
	const [workspaces] = useFetchWorkspaces();
	useFetchUserPermissions();
	const navigate = useNavigate();

	createEffect(() => {
		if (workspaces.state === "ready") {
			console.log("workspaces:", workspaces());
			const workspaceLength = workspaces()?.workspaces?.length || 0;
			if (workspaceLength === 0) {
				navigate("/onboard", { replace: true });
			}
		}
	});

	return <>{props.children}</>;
};

export default function WorkspacedRoutes() {
	return (
		<Route path="/" component={WorkspacedLayout}>
			<Route path="/" component={HomePage} />
			<Route path="/deployments">
				<Route path="/" component={ListDeploymentsPage} />
				<Route path="/new" component={CreateDeploymentPage} />
				<Route path="/:id" component={DeploymentInfo} />
			</Route>
			<Route path="/runners">
				<Route path="/" component={ListRunnersPage} />
				<Route path="/new" component={CreateRunnerPage} />
			</Route>
			<Route path="/domains">
				<Route path="/" component={ListDomainsPage} />
				<Route path="/new" component={CreateDomainPage} />
				<Route path="/:id" component={DomainInfo} />
			</Route>
			<Route path="/workspace">
				<Route path="/" component={General} />
				<Route path="/members" component={ManageWorkspace} />
				<Route path="/roles">
					<Route path="/" component={ManageRoles} />
					<Route path="/new" component={CreateRoles} />
					<Route path="/:roleId" component={RoleInfo} />
				</Route>
			</Route>
			<Route path="/container-registry">
				<Route path="/" component={ListContainerRepositories} />
				<Route path="/new" component={CreateContainerRepository} />
				<Route path="/:id" component={ContainerRepositoryInfo} />
			</Route>
		</Route>
	);
}
