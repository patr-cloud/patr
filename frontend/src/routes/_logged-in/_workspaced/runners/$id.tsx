import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { useQueryClient } from "@tanstack/solid-query";
import { createSignal, ErrorBoundary, Match, Show, Switch } from "solid-js";
import {
	Button,
	ButtonVariant,
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
import { useRunnerInfoQuery } from "~/hooks/fetch";
import { runnerKeys } from "~/hooks/query-keys";
import { DeleteRunnerResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import { Color } from "~/utils/color";
import RunnerDeployments from "./-components/deployments";
import RunnerMetrics from "./-components/metrics";
import RunnerLogs from "./-components/logs";
import RunnerInfo from "./-components/info";

const RunnerDetail = () => {
	const params = Route.useParams();
	const search = Route.useSearch();
	const tab = () => search().tab;

	const navigate = useNavigate();
	const toast = useToast();
	const queryClient = useQueryClient();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);

	const isAllowedResource = useIsAllowed("runner", "view", params().id);
	const runnerPermissions = useGetPermissions("runner", () => params().id);

	const runnerQuery = useRunnerInfoQuery(() => params().id);

	const { execute: deleteRunner, isLoading: isDeletingRunner } = createAuthenticatedAction(
		async ({ workspaceId: wsId }) => {
			const response = await httpRequest<DeleteRunnerResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/${params().id}`,
				{ method: "DELETE" }
			);

			if (!response.ok) {
				if (response.data.error === "resourceInUse") {
					toast("Cannot delete runner: it still has deployments", "error");
					return;
				}
				toast("Failed to delete runner", "error");
				return;
			}

			queryClient.invalidateQueries({ queryKey: runnerKeys.all(wsId) });
			toast("Runner deleted successfully", "success");
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
										label: runnerQuery.data?.runner.name ?? "Loading...",
									},
								]}
								subText="View deployments, system metrics, and logs for this runner."
								class="justify-between items-center"
								actions={() => (
									<div class="flex items-center gap-4">
										<StatusChip
											status={runnerQuery.data?.runner.connected ? "connected" : "unreachable"}
											size="md"
										/>
										<Show when={runnerPermissions().delete && runnerQuery.data?.runner.name}>
											<DeleteModal
												isLoading={isDeletingRunner()}
												title="Do You Really Want to Delete This Runner?"
												resourceName={runnerQuery.data!.runner.name}
												isOpen={isDeleteModalOpen}
												setIsOpen={setIsDeleteModalOpen}
												onClickDelete={(e) => {
													e.preventDefault();
													deleteRunner();
												}}
												renderTrigger={(open) => (
													<Button
														class="h-10"
														onClick={() => open?.(true)}
														variant={ButtonVariant.Outlined}
														color={Color.Error}
													>
														Delete
													</Button>
												)}
											/>
										</Show>
									</div>
								)}
								bottomContent={() => (
									<HeadTab
										tab={tab}
										tabItems={[
											{
												label: "Info",
												value: "info",
												onClick: (value) =>
													navigate({
														to: "/runners/$id",
														params: { id: params().id },
														search: { tab: value },
													}),
											},
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
										]}
									/>
								)}
							/>

							<PageContainerBody class="flex flex-col justify-between gap-8">
								<Switch fallback={<div class="text-grey text-sm py-8 text-center">No such tab</div>}>
									<Match when={tab() === "info"}>
										<div class="flex flex-col gap-6 w-full">
											<h2 class="text-lg text-white font-semibold">Details</h2>
											<RunnerInfo runnerId={params().id} />

											<div class="border-t border-border-color w-full mt-2" />

											<h2 class="text-lg text-white font-semibold">Deployments</h2>
											<RunnerDeployments runnerId={params().id} />
										</div>
									</Match>
									<Match when={tab() === "metrics"}>
										<RunnerMetrics runnerId={params().id} />
									</Match>
									<Match when={tab() === "logs"}>
										<RunnerLogs runnerId={params().id} />
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
		tab: (search.tab as string) || "info",
	}),
	component: RunnerDetail,
});
