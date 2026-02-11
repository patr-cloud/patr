import { A, useNavigate } from "@solidjs/router";
import { createMemo, createResource, createSignal, ErrorBoundary, Suspense } from "solid-js";
import { FiCheck, FiCopy } from "solid-icons/fi";
import { ListDeploymentResponse, WithId, Deployment } from "~/bindings";
import {
	Button,
	ButtonVariant,
	Link,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	Table,
	useToast,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import useIsAllowed from "~/hooks/use-is-allowed";

export const CopyButton = (props: { text: string }) => {
	const [copied, setCopied] = createSignal(false);

	const handleCopy = async (e: MouseEvent) => {
		e.stopPropagation(); // Prevent row click navigation
		try {
			await navigator.clipboard.writeText(props.text);
			setCopied(true);
			setTimeout(() => setCopied(false), 2000);
		} catch (error) {
			console.error("Failed to copy:", error);
		}
	};

	return (
		<button
			onClick={handleCopy}
			class="ml-2 p-1 rounded hover:bg-white/10 transition-colors"
			title={copied() ? "Copied!" : "Copy ID"}
		>
			{copied() ? <FiCheck size={14} class="text-gray-400" /> : <FiCopy size={14} class="text-gray-400" />}
		</button>
	);
};

const DeploymentListRow = (props: { item: WithId<Deployment> }) => {
	const navigate = useNavigate();

	return (
		<tr
			onClick={() => {
				navigate(`/deployments/${props.item.id}`);
			}}
			class="table-row"
		>
			<td class="flex-4 flex items-center justify-center">
				<span class="truncate">{props.item.id}</span>
				<CopyButton text={props.item.id} />
			</td>
			<td class="flex-4 flex items-center justify-center">{props.item.name}</td>
			<td class="flex-4 flex items-center justify-center">{props.item.status}</td>
			<td class="flex-4 flex items-center justify-center">{props.item.runner}</td>
		</tr>
	);
};

const ListDeploymentsPage = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const isAllowedCreate = useIsAllowed("deployment", "create", undefined, false);
	console.log("User permissions for creating deployment:", isAllowedCreate());

	const fetchParams = createMemo(() => {
		return [authState(), workspaceId()] as const;
	});

	const [deployments] = createResource(fetchParams, async ([auth, wsId]) => {
		if (!wsId || !auth || auth.type !== "LoggedIn") {
			return { deployments: [] };
		}

		console.log("Fetching deployments with workspace ID:", wsId);

		const response = await httpRequest<ListDeploymentResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/deployment`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch deployments:", response.data.error);
			toast("Failed to fetch deployments", "error");
			return { deployments: [] };
		}

		console.log("Fetched deployments:", response.data);

		// Fetch deployments logic goes here
		return { deployments: response.data.deployments };
	});

	return (
		<PageContainer>
			<PageContainerHead
				title="Deployments"
				subTitle="All Deployments"
				actions={() =>
					isAllowedCreate() && (
						<Link href="/deployments/new" buttonVariant={ButtonVariant.Contained} external={false}>
							CREATE NEW DEPLOYMENT
						</Link>
					)
				}
			/>

			<PageContainerBody>
				<ErrorBoundary
					fallback={(err, reset) => (
						<div>
							<p>Error loading deployments: {err.message}</p>
							<button onClick={reset}>Retry</button>
						</div>
					)}
				>
					<Suspense fallback={<div>Loading deployments...</div>}>
						<Table
							column_grids={["flex-4", "flex-4", "flex-4", "flex-4"]}
							rows={deployments()?.deployments || []}
							headings={["ID", "Deployment Name", "Status", "Runner"]}
							renderRow={(item) => <DeploymentListRow item={item} />}
						/>
					</Suspense>
				</ErrorBoundary>
			</PageContainerBody>
		</PageContainer>
	);
};

export default ListDeploymentsPage;
