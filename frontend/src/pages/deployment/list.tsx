import { useNavigate } from "@solidjs/router";
import { createMemo, createResource, ErrorBoundary, Show, Suspense } from "solid-js";
import { ListDeploymentResponse, WithId, Deployment, ListRunnersForWorkspaceResponse } from "~/bindings";
import {
	ButtonVariant,
	CopyableField,
	CopyableFieldVariant,
	EmptyState,
	Link,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Pagination,
	Table,
	useToast,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { useIsAllowed, createPaginationState } from "~/hooks";

const DeploymentListRow = (props: { item: WithId<Deployment>; runnerName: string }) => {
	const navigate = useNavigate();

	return (
		<tr
			onClick={() => {
				navigate(`/deployments/${props.item.id}`);
			}}
			class="table-row cursor-pointer"
		>
			<td class="flex-3 flex items-start justify-center min-w-0">
				<CopyableField
					variant={CopyableFieldVariant.Text}
					value={props.item.id}
					class="truncate"
					innerClass="text-white"
					buttonPosition="start"
				/>
			</td>
			<td class="flex-3 flex items-start justify-center min-w-0">
				<span class="truncate">{props.item.name}</span>
			</td>
			<td class="flex-3 flex items-start justify-center min-w-0">
				<span class="truncate">{props.item.status}</span>
			</td>
			<td class="flex-3 flex items-start justify-center min-w-0">
				<span class="truncate">{props.runnerName}</span>
			</td>
			<td class="flex-3 flex items-start justify-center min-w-0">
				<span class="truncate">{props.item.imageTag}</span>
			</td>
		</tr>
	);
};

const ListDeploymentsPage = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const isAllowedCreate = useIsAllowed("deployment", "create", undefined);
	const pagination = createPaginationState();

	const fetchParamsForDeployment = createMemo(() => {
		return [authState(), workspaceId(), pagination.page(), pagination.count()] as const;
	});

	const [deployments] = createResource(fetchParamsForDeployment, async ([auth, wsId, page, count]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { deployments: [] };
		}

		const response = await httpRequest<ListDeploymentResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment?page=${page}&count=${count}`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch deployments:", response.data.error);
			toast("Failed to fetch deployments", "error");
			return { deployments: [] };
		}

		pagination.setTotalCount(Number(response.headers.get("x-total-count") ?? 0));

		return { deployments: response.data.deployments };
	});

	const fetchParamsForRunners = createMemo(() => {
		return [authState(), workspaceId(), pagination.page(), pagination.count()] as const;
	});

	const [runners] = createResource(fetchParamsForRunners, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { runners: [] };
		}
		const response = await httpRequest<ListRunnersForWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch runners:", response.data.error);
			toast("Failed to fetch runners", "error");

			return { runners: [] };
		}

		console.log("Fetched runners:", response.data);
		return response.data;
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
				actions={() => {
					if (!isAllowedCreate()) return null;
					return (
						<Link href="/deployments/new" buttonVariant={ButtonVariant.Plain} external={false}>
							Create Deployment
						</Link>
					);
				}}
			/>

			<PageContainerBody class="flex flex-col justify-between">
				<ErrorBoundary
					fallback={(err, reset) => (
						<div>
							<p>Error loading deployments: {err.message}</p>
							<button onClick={reset}>Retry</button>
						</div>
					)}
				>
					<Suspense fallback={<div>Loading deployments...</div>}>
						<Show
							when={(deployments()?.deployments?.length ?? 0) > 0}
							fallback={<EmptyState title="No Deployments Added" />}
						>
							<Table
								column_grids={["flex-3", "flex-3", "flex-3", "flex-3", "flex-3"]}
								rows={deployments()?.deployments || []}
								headings={["ID", "Deployment Name", "Status", "Runner", "Image Tag"]}
								renderRow={(item) => (
									<DeploymentListRow
										item={item}
										runnerName={runners()?.runners.find((r) => r.id === item.runner)?.name ?? "Unknown"}
									/>
								)}
							/>
							<Pagination
								state={pagination}
								loading={deployments.loading}
								showPageSizeSelector={false}
								showGoToPage={false}
							/>
						</Show>
					</Suspense>
				</ErrorBoundary>
			</PageContainerBody>
		</PageContainer>
	);
};

export default ListDeploymentsPage;
