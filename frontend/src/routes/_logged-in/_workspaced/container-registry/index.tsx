import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, ErrorBoundary, Show, Suspense } from "solid-js";
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
	TableRow,
	TableCell,
} from "~/components";
import { createPaginationState, useIsAllowed } from "~/hooks";
import { WithId, ContainerRepository } from "~/bindings";
import { useContainerRegistriesQuery } from "~/hooks/fetch";
import { formatRelativeTime, formatSize } from "~/utils/func";

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
								<Table
									column_grids={["flex-4", "flex-3", "flex-2", "flex-3"]}
									headings={["Repository", "Last Updated", "Size", "Created"]}
									rows={repositoriesQuery.data?.repositories || []}
									renderRow={(repo: WithId<ContainerRepository>) => (
										<TableRow
											onClick={() => navigate({ to: `/container-registry/${repo.id}` })}
											aria-label={`Open repository ${repo.name}`}
										>
											<TableCell index={0}>
												<span class="truncate font-medium text-white">{repo.name}</span>
											</TableCell>
											<TableCell index={1}>
												{formatRelativeTime(repo.lastUpdated)}
											</TableCell>
											<TableCell index={2}>
												{formatSize(repo.size)}
											</TableCell>
											<TableCell index={3}>
												{formatRelativeTime(repo.created)}
											</TableCell>
										</TableRow>
									)}
								/>
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
