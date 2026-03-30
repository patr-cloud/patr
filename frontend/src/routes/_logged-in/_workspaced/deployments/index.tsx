import { createFileRoute } from "@tanstack/solid-router";
import { useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, createResource, ErrorBoundary, Show, Suspense } from "solid-js";
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

	if ("imageName" in props.item) {
		const fullImage = () =>
			`${props.item.registry}/${(props.item as { imageName: string }).imageName}:${props.item.imageTag}`;
		return (
			<Tooltip content={fullImage()} class="min-w-0">
				<span class="truncate font-log text-xs text-grey block">{fullImage()}</span>
			</Tooltip>
		);
	}

	const patrItem = props.item as WithId<Deployment> & { repositoryId: string };

	const [repoInfo] = createResource(
		() => [workspaceId(), patrItem.repositoryId] as const,
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

	const fullImage = () =>
		`registry.patr.cloud/${workspaceId()}/${repoInfo()?.repository.name ?? "..."}:${props.item.imageTag}`;

	return (
		<Tooltip content={fullImage()} class="min-w-0">
			<span class="truncate font-log text-xs text-grey block">
				<Show when={!repoInfo.loading} fallback={<span class="animate-pulse">{fullImage()}</span>}>
					{fullImage()}
				</Show>
			</span>
		</Tooltip>
	);
};

const DeploymentListRow = (props: { item: WithId<Deployment>; runnerName: string }) => {
	const navigate = useNavigate();

	return (
		<tr
			onClick={() => {
				navigate({ to: `/deployments/${props.item.id}` });
			}}
			class="table-row cursor-pointer"
		>
			<td class="flex-3 flex items-center justify-center min-w-0">
				<CopyableField
					variant={CopyableFieldVariant.Text}
					value={props.item.id}
					class="truncate"
					innerClass="text-white"
					buttonPosition="start"
				/>
			</td>
			<td class="flex-3 flex items-center justify-center min-w-0">
				<span class="truncate">{props.item.name}</span>
			</td>
			<td class="flex-2 flex items-center justify-center min-w-0">
				<StatusChip status={props.item.status} />
			</td>
			<td class="flex-3 flex items-center justify-center min-w-0">
				<span class="truncate">{props.runnerName}</span>
			</td>
			<td class="flex-4 flex items-center justify-start min-w-0">
				<ImageName item={props.item} />
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
									column_grids={["flex-3", "flex-3", "flex-2", "flex-3", "flex-4"]}
									rows={deployments()?.deployments || []}
									headings={["ID", "Name", "Status", "Runner", "Image"]}
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
