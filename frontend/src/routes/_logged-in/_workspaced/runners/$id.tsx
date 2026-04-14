import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { ErrorBoundary, Match, Show, Switch } from "solid-js";
import {
	HeadTab,
	LoadingSpinner,
	NoPermissionsPage,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
} from "~/components";
import useIsAllowed from "~/hooks/is-allowed";
import { useRunnerInfoQuery } from "~/hooks/fetch";
import RunnerDeployments from "./-components/deployments";
import RunnerMetrics from "./-components/metrics";
import RunnerLogs from "./-components/logs";

const RunnerDetail = () => {
	const params = Route.useParams();
	const search = Route.useSearch();
	const tab = () => search().tab;

	const navigate = useNavigate();

	const isAllowedResource = useIsAllowed("runner", "view", params().id);

	const runnerQuery = useRunnerInfoQuery(() => params().id);

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
									<div class="flex items-center gap-sm">
										<span
											class={`inline-flex items-center gap-xxs text-xs font-medium px-sm py-1 rounded-xl ${
												runnerQuery.data?.runner.connected
													? "bg-success/15 text-success"
													: "bg-error/15 text-error"
											}`}
										>
											<span
												class={`inline-block w-1.5 h-1.5 rounded-full ${
													runnerQuery.data?.runner.connected ? "bg-success" : "bg-error"
												}`}
											/>
											{runnerQuery.data?.runner.connected ? "Online" : "Unreachable"}
										</span>
									</div>
								)}
								bottomContent={() => (
									<HeadTab
										tab={tab}
										tabItems={[
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
									<Match when={tab() === "deployments"}>
										<RunnerDeployments runnerId={params().id} />
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
		tab: (search.tab as string) || "deployments",
	}),
	component: RunnerDetail,
});
