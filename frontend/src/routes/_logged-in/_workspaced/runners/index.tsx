import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createEffect, Show, ErrorBoundary, For, Suspense } from "solid-js";
import { WithId } from "~/bindings";
import type { Runner } from "~/bindings/Runner";
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
	Table,
	StatusChip,
} from "~/components";
import { useIsAllowed, createPaginationState } from "~/hooks";
import { useRunnersListQuery } from "~/hooks/fetch";
import { formatRelativeTime } from "~/utils/func";

const RunnerCard = (props: { item: WithId<Runner> }) => {
	const navigate = useNavigate();
	const goToDetail = () =>
		navigate({
			to: "/runners/$id",
			params: { id: props.item.id },
			search: { tab: "deployments" },
		});

	const lastSeenText = () =>
		props.item.connected ? "-" : props.item.lastSeen ? formatRelativeTime(props.item.lastSeen) : "Never";

	return (
		<article
			role="button"
			tabIndex={0}
			aria-label={`Open runner ${props.item.name}`}
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
				<StatusChip status={props.item.connected ? "connected" : "unreachable"} />
			</div>
			<dl class="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1 text-xs text-grey">
				<dt>Last Seen</dt>
				<dd class="text-white truncate">{lastSeenText()}</dd>
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

const ListRunnersPage = () => {
	const isAllowedCreate = useIsAllowed("runner", "create");
	const navigate = useNavigate();
	const search = Route.useSearch();
	const pagination = createPaginationState({
		search: () => search(),
		navigate,
	});

	const runnersQuery = useRunnersListQuery(
		() => search().page,
		() => search().count
	);

	createEffect(() => {
		const totalCount = runnersQuery.data?.totalCount;
		if (totalCount !== undefined) {
			pagination.setTotalCount(totalCount);
		}
	});

	return (
		<>
			<Title>Runners | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Runners",
						},
					]}
					subText="Runners execute deployments on your machines or clusters"
					actions={() => (
						<Show
							when={
								isAllowedCreate() &&
								runnersQuery.isSuccess &&
								(runnersQuery.data?.runners?.length ?? 0) > 0
							}
						>
							<Link href="/runners/new" buttonVariant={ButtonVariant.Outlined} external={false}>
								Add Runner
							</Link>
						</Show>
					)}
				/>
				<PageContainerBody class="flex flex-col justify-between">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading runners: {err.message}</p>
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
									<span class="text-sm">Loading runners...</span>
								</div>
							}
						>
							<Show
								when={(runnersQuery.data?.runners?.length ?? 0) > 0}
								fallback={
									<EmptyState
										title="No Runners Added"
										description={
											isAllowedCreate()
												? "Connect your infrastructure to start deploying."
												: undefined
										}
										action={
											isAllowedCreate() ? (
												<Link
													href="/runners/new"
													buttonVariant={ButtonVariant.Outlined}
													external={false}
												>
													Add Runner
												</Link>
											) : undefined
										}
									/>
								}
							>
								<div class="md:hidden flex flex-col gap-2">
									<For each={runnersQuery.data?.runners || []}>
										{(item) => <RunnerCard item={item} />}
									</For>
								</div>
								<div class="hidden md:block">
									<Table
										column_grids={["flex-4", "flex-2", "flex-3", "flex-3"]}
										rows={runnersQuery.data?.runners || []}
										headings={["Name", "Status", "Last Seen", "ID"]}
										renderRow={(item) => {
											const goToDetail = () =>
												navigate({
													to: "/runners/$id",
													params: { id: item.id },
													search: { tab: "deployments" },
												});
											return (
												<tr
													role="row"
													tabIndex={0}
													class="table-row cursor-pointer focus-visible:outline-primary"
													onClick={goToDetail}
													onKeyDown={(e) => {
														if (e.key === "Enter" || e.key === " ") {
															e.preventDefault();
															goToDetail();
														}
													}}
												>
													<td role="cell" class="flex-4 flex items-center justify-start min-w-0">
														<span class="truncate font-medium text-white">{item.name}</span>
													</td>
													<td role="cell" class="flex-2 flex items-center justify-center min-w-0">
														<StatusChip status={item.connected ? "connected" : "unreachable"} />
													</td>
													<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
														<span class="text-grey">
															{item.connected
																? "-"
																: item.lastSeen
																	? formatRelativeTime(item.lastSeen)
																	: "Never"}
														</span>
													</td>
													<td role="cell" class="flex-3 flex items-center justify-start min-w-0">
														<CopyableField
															variant={CopyableFieldVariant.Text}
															value={item.id}
															class="truncate"
															innerClass="text-grey font-log text-xs"
														/>
													</td>
												</tr>
											);
										}}
									/>
								</div>
								<Pagination
									state={pagination}
									loading={runnersQuery.isFetching}
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

export const Route = createFileRoute("/_logged-in/_workspaced/runners/")({
	validateSearch: (search: Record<string, unknown>): { page?: string; count?: string } => ({
		page: (search.page as string) || undefined,
		count: (search.count as string) || undefined,
	}),
	component: ListRunnersPage,
});
