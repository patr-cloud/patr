import { useNavigate } from "@tanstack/solid-router";
import { createEffect, ErrorBoundary, Show } from "solid-js";
import {
	CopyableField,
	CopyableFieldVariant,
	EmptyState,
	Pagination,
	StatusChip,
	Table,
	TableRow,
	TableCell,
} from "~/components";
import { createPaginationState } from "~/hooks";
import { useRunnerDeploymentsQuery } from "~/hooks/fetch";
import DeploymentImageName from "~/components/deployment-image-name";

interface RunnerDeploymentsProps {
	runnerId: string;
}

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
				<Table
					column_grids={["flex-3", "flex-3", "flex-2", "flex-4"]}
					rows={deploymentsQuery.data?.deployments || []}
					headings={["ID", "Name", "Status", "Image"]}
					renderRow={(item) => (
						<TableRow
							onClick={() => navigate({ to: `/deployments/${item.id}` })}
							aria-label={`Open deployment ${item.name}`}
						>
							<TableCell index={0} align="center">
								<CopyableField
									variant={CopyableFieldVariant.Text}
									value={item.id}
									class="truncate"
									innerClass="text-white"
									buttonPosition="start"
								/>
							</TableCell>
							<TableCell index={1} align="center">
								<span class="truncate">{item.name}</span>
							</TableCell>
							<TableCell index={2} align="center">
								<StatusChip status={item.status} />
							</TableCell>
							<TableCell index={3}>
								<DeploymentImageName item={item} />
							</TableCell>
						</TableRow>
					)}
				/>
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
