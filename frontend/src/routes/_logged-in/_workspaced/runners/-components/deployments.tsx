import { useNavigate } from "@tanstack/solid-router";
import { createMemo, createResource, ErrorBoundary, Show, Suspense } from "solid-js";
import { Deployment, GetContainerRepositoryInfoResponse, ListDeploymentResponse, WithId } from "~/bindings";
import {
	CopyableField,
	CopyableFieldVariant,
	EmptyState,
	Pagination,
	StatusChip,
	Table,
	Tooltip,
	useToast,
} from "~/components";
import { createPaginationState, useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

interface RunnerDeploymentsProps {
	runnerId: string;
}

const ImageName = (props: { item: WithId<Deployment> }) => {
	const [workspaceId] = useLastWorkspaceId();
	const isExternal = () => "imageName" in props.item;
	const repositoryId = () => (props.item as { repositoryId?: string }).repositoryId;

	const [repoInfo] = createResource(
		() => (isExternal() ? null : ([workspaceId(), repositoryId()] as const)),
		async (params) => {
			if (!params) return undefined;
			const [wsId, repoId] = params;
			if (!wsId || !repoId) return undefined;
			const response = await httpRequest<GetContainerRepositoryInfoResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}`,
				{ method: "GET" }
			);
			if (!response.ok) return undefined;
			return response.data;
		}
	);

	const fullImage = () => {
		if (isExternal()) {
			return `${props.item.registry}/${(props.item as { imageName: string }).imageName}:${props.item.imageTag}`;
		}
		return `registry.patr.cloud/${workspaceId()}/${repoInfo()?.repository.name ?? "..."}:${props.item.imageTag}`;
	};

	return (
		<Tooltip content={fullImage()} class="min-w-0">
			<span class="truncate font-log text-xs text-grey block">
				<Show
					when={isExternal() || !repoInfo.loading}
					fallback={<span class="animate-pulse">{fullImage()}</span>}
				>
					{fullImage()}
				</Show>
			</span>
		</Tooltip>
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
						column_grids={["flex-3", "flex-3", "flex-2", "flex-4"]}
						rows={deployments()?.deployments || []}
						headings={["ID", "Name", "Status", "Image"]}
						renderRow={(item) => (
							<tr
								onClick={() => navigate({ to: `/deployments/${item.id}` })}
								class="table-row cursor-pointer"
							>
								<td class="flex-3 flex items-center justify-center min-w-0">
									<CopyableField
										variant={CopyableFieldVariant.Text}
										value={item.id}
										class="truncate"
										innerClass="text-white"
										buttonPosition="start"
									/>
								</td>
								<td class="flex-3 flex items-center justify-center min-w-0">
									<span class="truncate">{item.name}</span>
								</td>
								<td class="flex-2 flex items-center justify-center min-w-0">
									<StatusChip status={item.status} />
								</td>
								<td class="flex-4 flex items-center justify-start min-w-0">
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
