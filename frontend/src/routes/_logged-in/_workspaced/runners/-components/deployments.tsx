import { useNavigate } from "@tanstack/solid-router";
import { ErrorBoundary, Show } from "solid-js";
import { createQuery } from "@tanstack/solid-query";
import { ListDeploymentResponse } from "~/bindings";
import { CopyableField, CopyableFieldVariant, EmptyState, Pagination, StatusChip, Table, useToast } from "~/components";
import { createPaginationState } from "~/hooks";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { runnerKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";
import DeploymentImageName from "~/components/deployment-image-name";

interface RunnerDeploymentsProps {
	runnerId: string;
}

const RunnerDeployments = (props: RunnerDeploymentsProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();
	const pagination = createPaginationState({
		search: () => ({}),
		navigate,
	});

	const deploymentsQuery = createQuery(() => {
		const auth = authState();
		const wsId = workspaceId();
		const p = pagination.page();
		const c = pagination.count();
		return {
			queryKey: runnerKeys.deployments(wsId ?? "", props.runnerId, p, c),
			enabled: !!wsId && !!auth && auth.type === "LoggedIn" && !!props.runnerId,
			queryFn: async () => {
				const response = await httpRequest<ListDeploymentResponse>(
					`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment?search[runner]=${props.runnerId}&page=${p}&count=${c}`,
					{ method: "GET" }
				);

				if (!response.ok) {
					toast("Failed to fetch deployments for runner", "error");
					throw new Error(response.data.error);
				}

				pagination.setTotalCount(Number(response.headers.get("x-total-count") ?? 0));
				return response.data;
			},
		};
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
