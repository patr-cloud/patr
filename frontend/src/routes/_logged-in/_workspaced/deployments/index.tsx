import { createFileRoute } from "@tanstack/solid-router";
import { useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, createMemo, ErrorBoundary, For, Show, Suspense } from "solid-js";
import { Deployment, WithId } from "~/bindings";
import {
	Button,
	ButtonVariant,
	CopyableField,
	CopyableFieldVariant,
	EmptyState,
	Link,
	LoadingSpinner,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Pagination,
	StatusChip,
	Table,
} from "~/components";
import { useDeploymentsQuery, useRunnersQuery } from "~/hooks/fetch";
import { useIsAllowed, createPaginationState } from "~/hooks";
import DeploymentImageName from "~/components/deployment-image-name";const DeploymentCard = (props: { item: WithId<Deployment>; runnerName: string }) => {
	const navigate = useNavigate();
	const goToDetail = () => navigate({ to: `/deployments/${props.item.id}` });

	return (
		<article
			role="button"
			tabIndex={0}
			aria-label={`Open deployment ${props.item.name}`}
			onClick={goToDetail}
			onKeyDown={(e) => {
				if (e.key === "Enter" || e.key === " ") {
					e.preventDefault();
					goToDetail();
				}
			}}
			class="bg-secondary-light rounded-xs p-md border border-border-color cursor-pointer hover:bg-secondary-medium focus-visible:outline-2 focus-visible:outline-primary focus-visible:-outline-offset-2 transition-colors"
		>
			<div class="flex justify-between items-start gap-2 mb-2">
				<h3 class="font-medium text-white truncate min-w-0">{props.item.name}</h3>
				<StatusChip status={props.item.status} />
			</div>
			<dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs text-grey">
				<dt>Runner</dt>
				<dd class="text-white truncate">{props.runnerName}</dd>
				<dt>Image</dt>
				<dd class="text-white truncate">
					<DeploymentImageName item={props.item} />
				</dd>
				<dt>ID</dt>
				<dd class="min-w-0" onClick={(e) => e.stopPropagation()}>
					<CopyableField
						variant={CopyableFieldVariant.Text}
						value={props.item.id}
						class="truncate"
						innerClass="text-grey font-log text-xs"
					/>
				</dd>
			</dl>
		</article>
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
				<DeploymentImageName item={props.item} />
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
	const isAllowedCreate = useIsAllowed("deployment", "create", undefined);
	const navigate = useNavigate();
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});

	const deploymentsQuery = useDeploymentsQuery(
		() => search().page,
		() => search().count
	);
	const runnersQuery = useRunnersQuery();

	createEffect(() => {
		const totalCount = deploymentsQuery.data?.totalCount;
		if (totalCount !== undefined) {
			pagination.setTotalCount(totalCount);
		}
	});

	const runnerNameMap = createMemo(() => new Map((runnersQuery.data?.runners || []).map((r) => [r.id, r.name])));

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
						<Show
							when={
								isAllowedCreate() &&
								deploymentsQuery.isSuccess &&
								(deploymentsQuery.data?.deployments?.length ?? 0) > 0
							}
						>
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
								when={(deploymentsQuery.data?.deployments?.length ?? 0) > 0}
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
								<div class="md:hidden flex flex-col gap-2">
									<For each={deploymentsQuery.data?.deployments || []}>
										{(item) => (
											<DeploymentCard
												item={item}
												runnerName={runnerNameMap().get(item.runner) ?? "Unknown"}
											/>
										)}
									</For>
								</div>
								<div class="hidden md:block">
									<Table
										column_grids={["flex-3", "flex-2", "flex-2", "flex-3", "flex-2"]}
										rows={deploymentsQuery.data?.deployments || []}
										headings={["Name", "Status", "Runner", "Image", "ID"]}
										renderRow={(item) => (
											<DeploymentListRow
												item={item}
												runnerName={runnerNameMap().get(item.runner) ?? "Unknown"}
											/>
										)}
									/>
								</div>
								<Pagination
									state={pagination}
									loading={deploymentsQuery.isFetching}
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
