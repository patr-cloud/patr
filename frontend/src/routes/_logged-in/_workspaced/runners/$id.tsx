import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createMemo, createResource, ErrorBoundary, Match, Show, Suspense, Switch } from "solid-js";
import { GetRunnerInfoResponse } from "~/bindings";
import {
	HeadTab,
	NoPermissionsPage,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import useIsAllowed from "~/hooks/is-allowed";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import RunnerDeployments from "./-components/deployments";
import RunnerMetrics from "./-components/metrics";
import RunnerLogs from "./-components/logs";

const RunnerDetail = () => {
	const params = Route.useParams();
	const search = Route.useSearch();
	const tab = () => search().tab;

	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();

	const isAllowedResource = useIsAllowed("runner", "view", params().id);

	const resourceParams = createMemo(() => {
		return [authState(), workspaceId(), params().id] as const;
	});

	const [runnerInfo] = createResource(resourceParams, async ([auth, wsId, id]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn" || id === "") {
			return undefined;
		}
		const response = await httpRequest<GetRunnerInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner/${id}`,
			{
				method: "GET",
			}
		);
		if (!response.ok) {
			console.error("Failed to fetch runner info:", response.data.error);
			toast("Failed to fetch runner info", "error");
			return undefined;
		}
		return response.data;
	});

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
						<Suspense fallback={<div>Loading runner info...</div>}>
							<PageContainerHead
								breadcrumbs={[
									{
										label: "Runners",
										url: "/runners",
									},
									{
										label: runnerInfo() ? runnerInfo()!.runner.name : "Loading...",
									},
								]}
								subText="View deployments, system metrics, and logs for this runner."
								class="justify-between items-center"
								actions={() => (
									<div class="flex items-center gap-sm">
										<span
											class={`inline-flex items-center gap-xxs text-xs font-medium px-sm py-1 rounded-xl ${
												runnerInfo()?.runner.connected
													? "bg-success/15 text-success"
													: "bg-error/15 text-error"
											}`}
										>
											<span
												class={`inline-block w-1.5 h-1.5 rounded-full ${
													runnerInfo()?.runner.connected ? "bg-success" : "bg-error"
												}`}
											/>
											{runnerInfo()?.runner.connected ? "Online" : "Unreachable"}
										</span>
									</div>
								)}
								bottomContent={() => (
									<HeadTab
										tab={tab}
										tabItems={[
											{
												label: "Deployments",
												value: "",
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
								<Switch fallback={<RunnerDeployments runnerId={params().id} />}>
									<Match when={tab() === "metrics"}>
										<RunnerMetrics runnerId={params().id} />
									</Match>
									<Match when={tab() === "logs"}>
										<RunnerLogs runnerId={params().id} />
									</Match>
								</Switch>
							</PageContainerBody>
						</Suspense>
					</ErrorBoundary>
				</PageContainer>
			</Show>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/runners/$id")({
	validateSearch: (search: Record<string, unknown>): { tab: string } => ({
		tab: (search.tab as string) || "",
	}),
	component: RunnerDetail,
});
