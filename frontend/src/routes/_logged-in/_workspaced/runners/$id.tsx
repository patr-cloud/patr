import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { useQueryClient } from "@tanstack/solid-query";
import { Title } from "@solidjs/meta";
import { createSignal, ErrorBoundary, Match, Show, Switch } from "solid-js";
import {
	DeleteModal,
	HeadTab,
	LoadingSpinner,
	NoPermissionsPage,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	StatusChip,
	useToast,
} from "~/components";
import { createAuthenticatedAction } from "~/hooks";
import useIsAllowed, { useGetPermissions } from "~/hooks/is-allowed";
import { useRunnerInfoQuery, useApiVersionQuery } from "~/hooks/fetch";
import { runnerKeys } from "~/hooks/query-keys";
import { httpRequest } from "~/utils/http-request";
import RunnerDeployments from "./-components/deployments";
import RunnerMetrics from "./-components/metrics";
import RunnerLogs from "./-components/logs";

const RunnerDetail = () => {
	const params = Route.useParams();
	const search = Route.useSearch();
	const tab = () => search().tab;

	const navigate = useNavigate();
	const toast = useToast();
	const queryClient = useQueryClient();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);

	const isAllowedResource = useIsAllowed("runner", "view", params().id);
	const runnerPermissions = useGetPermissions("runner", () => params().id || "");

	const runnerQuery = useRunnerInfoQuery(() => params().id);
	const versionQuery = useApiVersionQuery();

	const runner = () => runnerQuery.data?.runner;

	const { execute: deleteRunner, isLoading: isDeletingRunner } = createAuthenticatedAction(
		async ({ workspaceId }) => {
			if (!runnerPermissions().delete) {
				toast("You do not have permission to delete this runner", "error");
				return;
			}

			const r = runner();
			if (!r) {
				toast("Runner information is not available", "error");
				return;
			}

			const resp = await httpRequest(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/runner/${r.id}`,
				{ method: "DELETE" }
			);
			if (!resp.ok) {
				toast("Failed to delete runner", "error");
				return;
			}

			toast("Runner deleted successfully", "success");
			queryClient.invalidateQueries({ queryKey: runnerKeys.list(workspaceId) });
			navigate({ to: "/runners" });
		}
	);

	return (
		<>
			<Title>Runner Details | Patr</Title>
			<Show
				when={isAllowedResource()}
				fallback={
					<NoPermissionsPage
						title="Can't View Resource"
						message="You do not have permission to view this runner."
					/>
				}
			>
				<PageContainer>
					<ErrorBoundary
						fallback={(err, reset) => (
							<div>
								<p>Error loading runner info: {err.message}</p>
								<button onClick={reset}>Retry</button>
							</div>
						)}
					>
						<Show
							when={!runnerQuery.isPending}
							fallback={
								<div class="flex items-center justify-center gap-2 py-16 text-grey">
									<LoadingSpinner size={20} />
									<span class="text-sm">Loading runner...</span>
								</div>
							}
						>
							<PageContainerHead
								breadcrumbs={[
									{
										label: "Runners",
										url: "/runners",
									},
									{
										label: runner()?.name ?? "Loading...",
									},
								]}
								subText="View deployments, system metrics, and logs for this runner."
								class="justify-between items-center"
								actions={() => (
									<div class="flex items-center gap-sm">
										<Show when={runner()}>
											<StatusChip
												status={runner()!.connected ? "connected" : "unreachable"}
												size="md"
											/>
										</Show>
										<Show when={runner() && !runner()!.connected && runnerPermissions().delete}>
											<DeleteModal
												isLoading={isDeletingRunner()}
												title="Do You Really Want to Delete This Runner?"
												resourceName={runner()?.name || ""}
												isOpen={isDeleteModalOpen}
												setIsOpen={setIsDeleteModalOpen}
												onClickDelete={(e) => {
													e.preventDefault();
													deleteRunner();
												}}
											/>
										</Show>
									</div>
								)}
								bottomContent={() => (
									<HeadTab
										tab={tab}
										tabItems={[
											{
												label: "Metrics",
												value: "metrics",
												onClick: (value) =>
													navigate({
														to: "/runners/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
											{
												label: "Logs",
												value: "logs",
												onClick: (value) =>
													navigate({
														to: "/runners/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
											{
												label: "Deployments",
												value: "deployments",
												onClick: (value) =>
													navigate({
														to: "/runners/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
										]}
									/>
								)}
							/>

							<PageContainerBody class="flex flex-col justify-between gap-8">
								<Switch fallback={<div class="text-grey text-sm py-8 text-center">No such tab</div>}>
									<Match when={tab() === "metrics"}>
										<Show when={runner()}>
											{(r) => (
												<RunnerMetrics
													runnerId={r().id}
													version={r().version}
													connected={r().connected}
													lastSeen={r().lastSeen}
													apiVersion={versionQuery.data?.version}
												/>
											)}
										</Show>
									</Match>
									<Match when={tab() === "logs"}>
										<RunnerLogs runnerId={params().id} />
									</Match>
									<Match when={tab() === "deployments"}>
										<RunnerDeployments runnerId={params().id} />
									</Match>
								</Switch>
							</PageContainerBody>
						</Show>
					</ErrorBoundary>
				</PageContainer>
			</Show>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/runners/$id")({
	validateSearch: (search: Record<string, unknown>): { tab: string } => ({
		tab: (search.tab as string) || "metrics",
	}),
	component: RunnerDetail,
});
