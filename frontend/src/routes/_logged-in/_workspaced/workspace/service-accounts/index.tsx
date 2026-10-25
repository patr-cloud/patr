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
	Pagination,
	Table,
} from "~/components";
import { createPaginationState, recoverFromOutOfBounds, useIsAllowed, paginationSearch } from "~/hooks";
import { useServiceAccountsQuery, useWorkspaceInfoQuery } from "~/hooks/fetch";
import WorkspaceHeader from "~/routes/_logged-in/_workspaced/workspace/-components/workspace-header";

const ListServiceAccounts = () => {
	const navigate = useNavigate();
	const isCreateAllowed = useIsAllowed("serviceAccount", "create");
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});

	const workspaceInfoQuery = useWorkspaceInfoQuery();
	const serviceAccountsQuery = useServiceAccountsQuery(
		() => search().page?.toString(),
		() => search().count?.toString()
	);

	createEffect(() => {
		const totalCount = serviceAccountsQuery.data?.totalCount;
		if (totalCount !== undefined) {
			pagination.setTotalCount(totalCount);
		}
	});

	recoverFromOutOfBounds(
		() => serviceAccountsQuery.isError,
		() => serviceAccountsQuery.error?.message,
		pagination
	);

	const openServiceAccount = (id: string) => navigate({ to: "/workspace/service-accounts/$id", params: { id } });

	return (
		<>
			<Title>Service Accounts | Patr</Title>
			<PageContainer>
				<WorkspaceHeader
					workspaceName={workspaceInfoQuery.data?.name}
					activeTab="serviceAccounts"
					actions={() => (
						<Show when={isCreateAllowed()}>
							<Link
								href="/workspace/service-accounts/new"
								buttonVariant={ButtonVariant.Plain}
								external={false}
							>
								Create Service Account
							</Link>
						</Show>
					)}
				/>
				<PageContainerBody class="flex flex-col justify-between">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading service accounts: {err.message}</p>
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
									<span class="text-sm">Loading service accounts...</span>
								</div>
							}
						>
							<Show
								when={(serviceAccountsQuery.data?.serviceAccounts.length ?? 0) > 0}
								fallback={
									<EmptyState
										title="No Service Accounts Created"
										description={
											isCreateAllowed()
												? "Create a service account to give CI pipelines and other automation their own token and roles."
												: undefined
										}
										action={
											isCreateAllowed() ? (
												<Link
													href="/workspace/service-accounts/new"
													buttonVariant={ButtonVariant.Outlined}
													external={false}
												>
													Create Service Account
												</Link>
											) : undefined
										}
									/>
								}
							>
								<Table
									column_grids={["flex-3", "flex-5"]}
									headings={["Name", "Description"]}
									rows={serviceAccountsQuery.data?.serviceAccounts || []}
									renderRow={(serviceAccount) => (
										<tr
											role="row"
											tabIndex={0}
											onClick={() => openServiceAccount(serviceAccount.id)}
											onKeyDown={(e) => {
												if (e.key === "Enter" || e.key === " ") {
													e.preventDefault();
													openServiceAccount(serviceAccount.id);
												}
											}}
											class="table-row cursor-pointer focus-visible:outline-primary"
										>
											<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
												<span class="truncate font-medium text-white">
													{serviceAccount.name}
												</span>
											</td>
											<td role="cell" class="flex-5 flex items-center justify-start min-w-0">
												<span
													class={
														serviceAccount.description
															? "truncate"
															: "truncate text-grey italic"
													}
												>
													{serviceAccount.description || "No description"}
												</span>
											</td>
										</tr>
									)}
								/>
								<Pagination
									state={pagination}
									loading={serviceAccountsQuery.isFetching}
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

export const Route = createFileRoute("/_logged-in/_workspaced/workspace/service-accounts/")({
	validateSearch: paginationSearch,
	component: ListServiceAccounts,
});
