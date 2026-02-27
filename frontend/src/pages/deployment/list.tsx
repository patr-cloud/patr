import { useNavigate } from "@solidjs/router";
import { createMemo, createResource, ErrorBoundary, Suspense } from "solid-js";
import { ListDeploymentResponse, WithId, Deployment } from "~/bindings";
import {
	ButtonVariant,
	CopyButton,
	Link,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Table,
	useToast,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import useIsAllowed from "~/hooks/use-is-allowed";

const DeploymentListRow = (props: { item: WithId<Deployment> }) => {
	const navigate = useNavigate();

	return (
		<tr
			onClick={() => {
				navigate(`/deployments/${props.item.id}`);
			}}
			class="table-row"
		>
			<td class="flex-4 flex items-center justify-center">{props.item.name}</td>
			<td class="flex-4 flex items-center justify-center">{props.item.status}</td>
			<td class="flex-4 flex items-center justify-center">{props.item.runner}</td>
			<td class="flex-4 flex items-center justify-center">{props.item.imageTag}</td>
		</tr>
	);
};

const ListDeploymentsPage = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const isAllowedCreate = useIsAllowed("deployment", "create", undefined, false);
	console.log("User permissions for creating deployment:", isAllowedCreate());

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId()] as const;
	});

	const [deployments] = createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { deployments: [] };
		}

		console.log("Fetching deployments with workspace ID:", wsId);

		const response = await httpRequest<ListDeploymentResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch deployments:", response.data.error);
			toast("Failed to fetch deployments", "error");
			return { deployments: [] };
		}

		console.log("Fetched deployments:", response.data);

		// Fetch deployments logic goes here
		return { deployments: response.data.deployments };
	});

	return (
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Deployments",
					},
				]}
				subText="A deployment represents a containerized application running on a runner."
				actions={() =>
					isAllowedCreate() && (
						<Link href="/deployments/new" buttonVariant={ButtonVariant.Plain} external={false}>
							New Deployment
						</Link>
					)
				}
			/>

			<PageContainerBody>
				<ErrorBoundary
					fallback={(err, reset) => (
						<div>
							<p>Error loading deployments: {err.message}</p>
							<button onClick={reset}>Retry</button>
						</div>
					)}
				>
					<Suspense fallback={<div>Loading deployments...</div>}>
						<Table
							column_grids={["flex-4", "flex-4", "flex-4", "flex-4"]}
							rows={deployments()?.deployments || []}
							headings={["Deployment Name", "Status", "Runner", "Image Tag"]}
							renderRow={(item) => <DeploymentListRow item={item} />}
						/>
					</Suspense>
				</ErrorBoundary>
			</PageContainerBody>
		</PageContainer>
	);
};

export default ListDeploymentsPage;
