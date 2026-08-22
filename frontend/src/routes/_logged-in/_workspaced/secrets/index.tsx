import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, ErrorBoundary, For, Suspense, Show } from "solid-js";
import {
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Table,
	Button,
	ButtonVariant,
	EmptyState,
	Link,
	Pagination,
	LoadingSpinner,
} from "~/components";
import { useIsAllowed, createPaginationState, recoverFromOutOfBounds } from "~/hooks";
import { useSecretsQuery } from "~/hooks/fetch";
import { cloudOnly } from "~/utils/env";
import { formatRelativeTime } from "~/utils/func";

const ListSecretsPage = () => {
	const navigate = useNavigate();

	const isCreateAllowed = useIsAllowed("secret", "create");
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});

	const secretsQuery = useSecretsQuery(
		() => search().page,
		() => search().count
	);

	createEffect(() => {
		const totalCount = secretsQuery.data?.totalCount;
		if (totalCount !== undefined) {
			pagination.setTotalCount(totalCount);
		}
	});

	recoverFromOutOfBounds(
		() => secretsQuery.isError,
		() => secretsQuery.error?.message,
		pagination
	);

	return (
		<>
			<Title>Secrets | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Secrets",
						},
					]}
					subText="Store sensitive values securely and reference them from your deployments."
					actions={() => (
						<Show
							when={
								isCreateAllowed() &&
								secretsQuery.isSuccess &&
								(secretsQuery.data?.secrets?.length ?? 0) > 0
							}
						>
							<Link href="/secrets/new" buttonVariant={ButtonVariant.Outlined} external={false}>
								Add Secret
							</Link>
						</Show>
					)}
				/>
				<PageContainerBody class="flex flex-col justify-between">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading secrets: {err.message}</p>
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
									<span class="text-sm">Loading secrets...</span>
								</div>
							}
						>
							<Show
								when={(secretsQuery.data?.secrets?.length ?? 0) > 0}
								fallback={
									<EmptyState
										title="No Secrets Added"
										description={
											isCreateAllowed()
												? "Store sensitive values securely and reference them from your deployments."
												: undefined
										}
										action={
											isCreateAllowed() ? (
												<Link
													href="/secrets/new"
													buttonVariant={ButtonVariant.Outlined}
													external={false}
												>
													Add Secret
												</Link>
											) : undefined
										}
									/>
								}
							>
								<div class="md:hidden flex flex-col gap-2">
									<For each={secretsQuery.data?.secrets || []}>
										{(item) => (
											<article
												role="button"
												tabIndex={0}
												aria-label={`Open secret ${item.name}`}
												onClick={() => navigate({ to: `/secrets/${item.id}` })}
												onKeyDown={(e) => {
													if (e.key === "Enter" || e.key === " ") {
														e.preventDefault();
														navigate({ to: `/secrets/${item.id}` });
													}
												}}
												class="bg-secondary-light rounded-xs p-md border border-border-color cursor-pointer hover:bg-secondary-medium focus-visible:outline-2 focus-visible:outline-primary focus-visible:-outline-offset-2 transition-colors"
											>
												<h3 class="font-medium text-white truncate min-w-0">{item.name}</h3>
												<dl class="mt-2 flex flex-col gap-1 text-xs text-grey">
													<div class="flex justify-between gap-2">
														<dt>Last updated</dt>
														<dd class="text-white truncate">
															{formatRelativeTime(item.lastUpdated)}
														</dd>
													</div>
													<div class="flex justify-between gap-2">
														<dt>Created</dt>
														<dd class="text-white truncate">
															{formatRelativeTime(item.created)}
														</dd>
													</div>
												</dl>
											</article>
										)}
									</For>
								</div>
								<div class="hidden md:block">
									<Table
										column_grids={["flex-5", "flex-3", "flex-3"]}
										rows={secretsQuery.data?.secrets || []}
										headings={["Name", "Last updated", "Created"]}
										renderRow={(item) => (
											<tr
												role="row"
												tabIndex={0}
												onClick={() => navigate({ to: `/secrets/${item.id}` })}
												onKeyDown={(e) => {
													if (e.key === "Enter" || e.key === " ") {
														e.preventDefault();
														navigate({ to: `/secrets/${item.id}` });
													}
												}}
												class="table-row cursor-pointer focus-visible:outline-primary"
											>
												<td role="cell" class="flex-5 flex items-center justify-start min-w-0">
													<span class="truncate font-medium text-white">{item.name}</span>
												</td>
												<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
													<span class="truncate text-grey">
														{formatRelativeTime(item.lastUpdated)}
													</span>
												</td>
												<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
													<span class="truncate text-grey">
														{formatRelativeTime(item.created)}
													</span>
												</td>
											</tr>
										)}
									/>
								</div>
								<Pagination
									state={pagination}
									loading={secretsQuery.isFetching}
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

export const Route = createFileRoute("/_logged-in/_workspaced/secrets/")(
	cloudOnly({
		validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
			page: (search.page as string) || undefined,
			count: (search.count as string) || undefined,
		}),
		component: ListSecretsPage,
	})
);
