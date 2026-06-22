import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, ErrorBoundary, For, Show, Suspense } from "solid-js";
import {
	Button,
	ButtonVariant,
	EmptyState,
	Link,
	LoadingSpinner,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Pagination,
	Table,
} from "~/components";
import { createPaginationState, useIsAllowed } from "~/hooks";
import { WithId, ContainerRepository } from "~/bindings";
import { useContainerRegistriesQuery } from "~/hooks/fetch";
import { formatRelativeTime, formatSize } from "~/utils/func";

const RepositoryCard = (props: { item: WithId<ContainerRepository> }) => {
	const navigate = useNavigate();
	const goToDetail = () => navigate({ to: `/container-registry/${props.item.id}` });

	return (
		<article
			role="button"
			tabIndex={0}
			aria-label={`Open repository ${props.item.name}`}
			onClick={goToDetail}
			onKeyDown={(e) => {
				if (e.key === "Enter" || e.key === " ") {
					e.preventDefault();
					goToDetail();
				}
			}}
			class="bg-secondary-light rounded-xs p-md border border-border-color cursor-pointer hover:bg-secondary-medium focus-visible:outline-2 focus-visible:outline-primary focus-visible:-outline-offset-2 transition-colors"
		>
			<h3 class="font-medium text-white truncate min-w-0 mb-2">{props.item.name}</h3>
			<dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs text-grey">
				<dt>Size</dt>
				<dd class="text-white truncate">{formatSize(props.item.size)}</dd>
				<dt>Last Updated</dt>
				<dd class="text-white truncate">{formatRelativeTime(props.item.lastUpdated)}</dd>
				<dt>Created</dt>
				<dd class="text-white truncate">{formatRelativeTime(props.item.created)}</dd>
			</dl>
		</article>
	);
};

const ListContainerRepositories = () => {
	const navigate = useNavigate();
	const isAllowedCreate = useIsAllowed("containerRegistryRepository", "create", undefined);
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});

	const repositoriesQuery = useContainerRegistriesQuery(
		() => search().page,
		() => search().count
	);

	createEffect(() => {
		const totalCount = repositoriesQuery.data?.totalCount;
		if (totalCount !== undefined) {
			pagination.setTotalCount(totalCount);
		}
	});

	return (
		<>
			<Title>Container Repositories | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Container Repositories",
						},
					]}
					subText="Store and manage container images for your deployments"
					actions={() => (
						<Show
							when={
								isAllowedCreate() &&
								repositoriesQuery.isSuccess &&
								(repositoriesQuery.data?.repositories?.length ?? 0) > 0
							}
						>
							<Link
								href="/container-registry/new"
								buttonVariant={ButtonVariant.Outlined}
								external={false}
							>
								Create Repository
							</Link>
						</Show>
					)}
				/>

				<PageContainerBody class="flex flex-col justify-between">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading repositories: {err.message}</p>
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
									<span class="text-sm">Loading repositories...</span>
								</div>
							}
						>
							<Show
								when={(repositoriesQuery.data?.repositories?.length ?? 0) > 0}
								fallback={
									<EmptyState
										title="No Container Repositories Yet"
										description={
											isAllowedCreate() ? "Create one to store your container images." : undefined
										}
										action={
											isAllowedCreate() ? (
												<Link
													href="/container-registry/new"
													buttonVariant={ButtonVariant.Outlined}
													external={false}
												>
													Create Repository
												</Link>
											) : undefined
										}
									/>
								}
							>
								<div class="md:hidden flex flex-col gap-2">
									<For each={repositoriesQuery.data?.repositories || []}>
										{(item) => <RepositoryCard item={item} />}
									</For>
								</div>
								<div class="hidden md:block">
									<Table
										column_grids={["flex-4", "flex-3", "flex-2", "flex-3"]}
										headings={["Repository", "Last Updated", "Size", "Created"]}
										rows={repositoriesQuery.data?.repositories || []}
										renderRow={(repo: WithId<ContainerRepository>) => (
											<tr
												role="row"
												tabIndex={0}
												class="table-row cursor-pointer focus-visible:outline-primary"
												onClick={() => navigate({ to: `/container-registry/${repo.id}` })}
												onKeyDown={(e) => {
													if (e.key === "Enter" || e.key === " ") {
														e.preventDefault();
														navigate({ to: `/container-registry/${repo.id}` });
													}
												}}
											>
												<td role="cell" class="flex-4 min-w-0">
													<span class="truncate font-medium text-white">{repo.name}</span>
												</td>
												<td role="cell" class="flex-3">
													{formatRelativeTime(repo.lastUpdated)}
												</td>
												<td role="cell" class="flex-2">
													{formatSize(repo.size)}
												</td>
												<td role="cell" class="flex-3">
													{formatRelativeTime(repo.created)}
												</td>
											</tr>
										)}
									/>
								</div>
								<Pagination
									state={pagination}
									loading={repositoriesQuery.isFetching}
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

export const Route = createFileRoute("/_logged-in/_workspaced/container-registry/")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: ListContainerRepositories,
});
