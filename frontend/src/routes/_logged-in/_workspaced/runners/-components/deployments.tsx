import { useNavigate } from "@tanstack/solid-router";
import { createEffect, ErrorBoundary, For, Show } from "solid-js";
import { Deployment, WithId } from "~/bindings";
import { CopyableField, CopyableFieldVariant, EmptyState, Pagination, StatusChip, Table } from "~/components";
import { createPaginationState } from "~/hooks";
import { useRunnerDeploymentsQuery } from "~/hooks/fetch";
import DeploymentImageName from "~/components/deployment-image-name";

interface RunnerDeploymentsProps {
	runnerId: string;
}

const DeploymentCard = (props: { item: WithId<Deployment> }) => {
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

const RunnerDeployments = (props: RunnerDeploymentsProps) => {
	const navigate = useNavigate();
	const pagination = createPaginationState({
		search: () => ({}),
		navigate,
	});

	const deploymentsQuery = useRunnerDeploymentsQuery(
		() => props.runnerId,
		() => pagination.page(),
		() => pagination.count()
	);

	createEffect(() => {
		const totalCount = deploymentsQuery.data?.totalCount;
		if (totalCount !== undefined) {
			pagination.setTotalCount(totalCount);
		}
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
			<Show
				when={(deploymentsQuery.data?.deployments?.length ?? 0) > 0}
				fallback={<EmptyState title="No Deployments on This Runner" />}
			>
				<div class="md:hidden flex flex-col gap-2">
					<For each={deploymentsQuery.data?.deployments || []}>
						{(item) => <DeploymentCard item={item} />}
					</For>
				</div>
				<div class="hidden md:block">
					<Table
						column_grids={["flex-3", "flex-3", "flex-2", "flex-4"]}
						rows={deploymentsQuery.data?.deployments || []}
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
									<DeploymentImageName item={item} />
								</td>
							</tr>
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
		</ErrorBoundary>
	);
};

export default RunnerDeployments;
