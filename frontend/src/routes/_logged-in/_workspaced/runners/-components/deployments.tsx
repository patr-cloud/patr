import { useNavigate } from "@tanstack/solid-router";
import { createMemo, createResource, ErrorBoundary, Show, Suspense } from "solid-js";
import { Deployment, GetContainerRepositoryInfoResponse, ListDeploymentResponse, WithId } from "~/bindings";
import { EmptyState, Pagination, StatusChip, Table, useToast } from "~/components";
import { createPaginationState, useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

interface RunnerDeploymentsProps {
	runnerId: string;
}

const ImageName = (props: { item: WithId<Deployment> }) => {
	const [workspaceId] = useLastWorkspaceId();

	if ("imageName" in props.item) {
		return (
			<span class="truncate font-log text-xs text-grey">
				{props.item.registry}/{props.item.imageName}:{props.item.imageTag}
			</span>
		);
	}

	const [repoInfo] = createResource(
		() => [workspaceId(), props.item.repositoryId] as const,
		async ([wsId, repoId]) => {
			if (!wsId || !repoId) return undefined;
			const response = await httpRequest<GetContainerRepositoryInfoResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}`,
				{ method: "GET" }
			);
			if (!response.ok) return undefined;
			return response.data;
		}
	);

	return (
		<span class="truncate font-log text-xs text-grey">
			registry.patr.cloud/{workspaceId()}/
			<Show when={!repoInfo.loading} fallback={<span class="animate-pulse">loading</span>}>
				{repoInfo()?.repository.name ?? "unknown"}
			</Show>
			:{props.item.imageTag}
		</span>
	);
};

const RunnerDeployments = (props: RunnerDeploymentsProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const pagination = createPaginationState({
		search: () => ({}),
		navigate,
	});

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId(), props.runnerId, pagination.page(), pagination.count()] as const;
	});

	const [deployments] = createResource(fetchParams, async ([auth, wsId, runnerId, page, count]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || !runnerId) {
			return { deployments: [] };
		}
		const response = await httpRequest<ListDeploymentResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment?runner=${runnerId}&page=${page}&count=${count}`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch deployments:", response.data.error);
			toast("Failed to fetch deployments for runner", "error");
			return { deployments: [] };
		}

		pagination.setTotalCount(Number(response.headers.get("x-total-count") ?? 0));
		return response.data;
	});

	return (
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
					fallback={<EmptyState title="No Deployments on This Runner" />}
				>
					<Table
						column_grids={["flex-3", "flex-2", "flex-5"]}
						rows={deployments()?.deployments || []}
						headings={["Name", "Status", "Image"]}
						renderRow={(item) => (
							<tr
								onClick={() => navigate({ to: `/deployments/${item.id}` })}
								class="table-row cursor-pointer"
							>
								<td class="flex-3 flex items-center justify-start min-w-0">
									<span class="truncate">{item.name}</span>
								</td>
								<td class="flex-2 flex items-center justify-center min-w-0">
									<StatusChip status={item.status} />
								</td>
								<td class="flex-5 flex items-center justify-start min-w-0">
									<ImageName item={item} />
								</td>
							</tr>
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
	);
};

export default RunnerDeployments;
