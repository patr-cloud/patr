import { createMemo, createResource, ErrorBoundary, Suspense } from "solid-js";
import { ListRunnersForWorkspaceResponse } from "~/bindings";
import { ButtonVariant, Link, PageContainer, PageContainerBody, PageContainerHead, Table } from "~/components";
import { useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { formatRelativeTime } from "~/utils/func";
import { httpRequest } from "~/utils/http-request";

const ListRunnersPage = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId()] as const;
	});

	const [runners] = createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { runners: [] };
		}
		const response = await httpRequest<ListRunnersForWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/runner`,
			{
				method: "GET",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch runners:", response.data.error);
			toast("Failed to fetch runners", "error");

			return { runners: [] };
		}

		console.log("Fetched runners:", response.data);
		return response.data;
	});

	return (
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Runners",
					},
				]}
				subText="Runners execute deployments on your machines or clusters"
				actions={() => (
					<Link href="/runners/new" buttonVariant={ButtonVariant.Plain} external={false}>
						Add Runner
					</Link>
				)}
			/>
			<PageContainerBody class="flex flex-col justify-between gap-8">
				<ErrorBoundary
					fallback={(err, reset) => (
						<div>
							<p>Error loading runners: {err.message}</p>
							<button onClick={reset}>Retry</button>
						</div>
					)}
				>
					<Suspense fallback={<div>Loading...</div>}>
						<Table
							column_grids={["flex-4", "flex-4", "flex-4"]}
							rows={runners()?.runners || []}
							headings={["Runner Name", "Status", "Last Seen"]}
							renderRow={(item) => (
								<tr
									class="border border-border-color min-h-10 cursor-pointer flex items-center justify-center w-full px-xl
                  bg-secondary-light last-of-type:rounded-b-xs"
								>
									<td class="flex items-center justify-center flex-1">{item.name}</td>
									<td class="flex items-center justify-center flex-1">
										{item.connected ? "Connected" : "Disconnected"}
									</td>
									<td class="flex items-center justify-center flex-1">
										{item.lastSeen ? formatRelativeTime(item.lastSeen) : "N/A"}
									</td>
								</tr>
							)}
						/>
					</Suspense>
				</ErrorBoundary>
			</PageContainerBody>
		</PageContainer>
	);
};

export default ListRunnersPage;
