import { createFileRoute } from "@tanstack/solid-router";
import { useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, createResource, ErrorBoundary, Show, Suspense } from "solid-js";
import { LoadingSpinner } from "~/components/loading-spinner";
import Button from "~/components/button";
import { Deployment, GetContainerRepositoryInfoResponse, ListDeploymentResponse, WithId } from "~/bindings";
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
	StatusChip,
	Table,
	Tooltip,
	useToast,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { useFetchRunners } from "~/hooks/fetch";
import { httpRequest } from "~/utils/http-request";
import { useIsAllowed, createPaginationState } from "~/hooks";

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

const DeploymentListRow = (props: { item: WithId<Deployment>; runnerName: string }) => {
	const navigate = useNavigate();

	const goToDetail = () => navigate({ to: `/deployments/${props.item.id}` });

	return (
		<tr
			role="row"
			tabIndex={0}
			onClick={goToDetail}
			onKeyDown={(e) => {
				if (e.key === "Enter" || e.key === " ") {
					e.preventDefault();
					goToDetail();
				}
			}}
			class="table-row cursor-pointer focus-visible:outline-primary"
		>
			<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
				<span class="truncate font-medium text-white">{props.item.name}</span>
			</td>
			<td role="cell" class="flex-2 flex items-center justify-center min-w-0">
				<StatusChip status={props.item.status} />
			</td>
			<td role="cell" class="flex-2 flex items-center justify-start min-w-0">
				<span class="truncate">{props.runnerName}</span>
			</td>
			<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
				<ImageName item={props.item} />
			</td>
			<td role="cell" class="flex-2 flex items-center justify-start min-w-0">
				<CopyableField
					variant={CopyableFieldVariant.Text}
					value={props.item.id}
					class="truncate"
					innerClass="text-grey font-log text-xs"
				/>
			</td>
		</tr>
	);
};

const ListDeploymentsPage = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const isAllowedCreate = useIsAllowed("deployment", "create", undefined);
	const navigate = useNavigate();
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});

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

	const [runners] = useFetchRunners();

	const runnerNameMap = createMemo(() => {
		return new Map((runners()?.runners || []).map((r) => [r.id, r.name]));
	});

	return (
		<>
			<Title>Deployments | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Deployments",
						},
					]}
					subText="A deployment represents a containerized application running on a runner."
					actions={() => (
						<Show when={isAllowedCreate() && (deployments()?.deployments?.length ?? 0) > 0}>
							<Link href="/deployments/new" buttonVariant={ButtonVariant.Outlined} external={false}>
								Create Deployment
							</Link>
						</Show>
					)}
				/>

				<PageContainerBody class="flex flex-col justify-between">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading deployments: {err.message}</p>
								<Button variant={ButtonVariant.Outlined} onClick={reset}>
									Retry
								</Button>
							</div>
						)}
					>
						<Suspense
							fallback={
								<div class="flex items-center justify-center gap-2 py-16 text-grey">
									<LoadingSpinner size={20} />
									<span class="text-sm">Loading deployments...</span>
								</div>
							}
						>
							<Show
								when={(deployments()?.deployments?.length ?? 0) > 0}
								fallback={
									<EmptyState
										title="No Deployments Added"
										description={
											isAllowedCreate()
												? "Deploy a containerized application to get started."
												: undefined
										}
										action={
											isAllowedCreate() ? (
												<Link
													href="/deployments/new"
													buttonVariant={ButtonVariant.Outlined}
													external={false}
												>
													Create Deployment
												</Link>
											) : undefined
										}
									/>
								}
							>
								<Table
									column_grids={["flex-3", "flex-2", "flex-2", "flex-3", "flex-2"]}
									rows={deployments()?.deployments || []}
									headings={["Name", "Status", "Runner", "Image", "ID"]}
									renderRow={(item) => (
										<DeploymentListRow
											item={item}
											runnerName={runnerNameMap().get(item.runner) ?? "Unknown"}
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
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/deployments/")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: ListDeploymentsPage,
});
