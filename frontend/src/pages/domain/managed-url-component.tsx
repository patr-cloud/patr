import { createSignal } from "solid-js";
import {
	GetDomainInfoInWorkspaceResponse,
	ManagedUrl,
	UpdateManagedURLRequest,
	UpdateManagedURLResponse,
	WithId,
} from "~/bindings";
import { Button, ButtonVariant, Input, InputDropdown, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import DeploymentOption from "./deployment-option";
import { domainTypeToTitle } from "./utils";
import { FiAlertCircle, FiEdit2, FiTrash, FiX } from "solid-icons/fi";
import InfoPopup from "~/components/info-popup";

type urlTypeT = "proxyUrl" | "redirect" | "proxyDeployment" | "proxyStaticSite";

interface ManageUrlRowProps {
	managedUrl: WithId<ManagedUrl>;
	domainInfo: GetDomainInfoInWorkspaceResponse;
	onUpdate: () => void;
}

const ManageUrlRow = (props: ManageUrlRowProps) => {
	const [openEdit, setOpenEdit] = createSignal(false);
	const [shouldDelete, setShouldDelete] = createSignal(false);

	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const onDelete = async (e: EventT<MouseEvent, HTMLButtonElement>) => {
		e.stopPropagation();

		const auth = authState();
		const wsId = workspaceId();

		if (!wsId || !auth || auth.type !== "LoggedIn") {
			toast("Authentication required", "error");
			return;
		}

		const response = await httpRequest<void>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/infrastructure/managed-url/${props.managedUrl.id}`,
			{
				method: "DELETE",
				headers: {
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);

		if (!response.ok) {
			console.error("Failed to delete managed URL:", response.data.error);
			toast("Failed to delete managed URL", "error");
			return;
		}

		toast("Managed URL deleted successfully", "success");
		props.onUpdate?.();
	};

	return (
		<>
			{openEdit() ? (
				<tr class="table-row">
					<td class="w-full" colspan={3}>
						<ManagedUrlComponent
							domainInfo={props.domainInfo}
							managedUrl={props.managedUrl}
							close={() => setOpenEdit(false)}
							onUpdate={() => props.onUpdate()}
						/>
					</td>
				</tr>
			) : (
				<tr class="table-row cursor-auto">
					<td class="flex-3 flex items-center justify-center">
						<a
							href={`https://${props.managedUrl.subDomain}.${props.domainInfo.name}${props.managedUrl.path}`}
							target="_blank"
							rel="noopener noreferrer"
						>
							{props.managedUrl.subDomain}.{props.domainInfo.name}
							{props.managedUrl.path}
						</a>
					</td>
					<td class="flex-3 flex items-center justify-center">{domainTypeToTitle(props.managedUrl.type)}</td>

					<td class="flex-[0.3] flex items-center justify-center">
						<div class="flex gap-2 items-center">
							{!props.managedUrl.isConfigured && (
								<InfoPopup
									triggerIcon={() => <FiAlertCircle color="text-warning" size={18} />}
									title="Configuration Required"
									content={() => (
										<div>
											<p class="text-gray-300 text-sm mb-2">
												This managed URL is not properly configured. Please update your DNS settings to point to our
												servers.
											</p>

											<div class="bg-black/30 p-2 rounded text-xs text-gray-400 mb-2">
												<p>Type: CNAME</p>
												<p>
													Name: {props.managedUrl.subDomain}.{props.domainInfo.name || ""}
												</p>
												<p>Value: ingress.patr.cloud</p>
											</div>
										</div>
									)}
								/>
							)}
							{shouldDelete() ? (
								<>
									<button onClick={onDelete} class="text-red-500">
										Delete
									</button>
									<button onClick={() => setShouldDelete(false)}>Cancel</button>
								</>
							) : (
								<>
									<button
										onClick={() => {
											console.log("Edit clicked");
											setOpenEdit(true);
										}}
										class="text-gray-400 hover:bg-white/10 p-1 rounded transition-colors cursor-pointer"
									>
										<FiEdit2 size={18} />
									</button>
									<button
										onClick={(e) => {
											e.stopPropagation();
											setShouldDelete(true);
										}}
										class="text-red-500 hover:bg-white/10 p-1 rounded transition-colors cursor-pointer"
									>
										<FiTrash size={18} />
									</button>
								</>
							)}
						</div>
					</td>
				</tr>
			)}
		</>
	);
};

interface ManagedUrlComponentProps {
	domainInfo: GetDomainInfoInWorkspaceResponse;
	managedUrl: WithId<ManagedUrl>;
	close: () => void;
	onUpdate: () => void;
}

const ManagedUrlComponent = (props: ManagedUrlComponentProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const [path, setPath] = createSignal(props.managedUrl.path);
	const [urlType, setUrlType] = createSignal<urlTypeT>(props.managedUrl.type as urlTypeT);
	const [target, setTarget] = createSignal<string | null>(
		props.managedUrl.type === "proxyDeployment" ? props.managedUrl.deploymentId : null
	);
	const [deploymentPort, setDeploymentPort] = createSignal<number | null>(
		props.managedUrl.type === "proxyDeployment" ? props.managedUrl.port : null
	);

	const onSubmit = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();

		const auth = authState();
		const wsId = workspaceId();

		if (!wsId || !auth || auth.type !== "LoggedIn") {
			toast("Authentication required", "error");
			return;
		}

		const urlTypeVal = urlType();
		const targetVal = target();
		if (!urlTypeVal || !targetVal) {
			toast("Please fill in all required fields", "error");
			return;
		}

		const requestBody: UpdateManagedURLRequest = {
			path: path(),
			type: "proxyDeployment",
			deploymentId: targetVal,
			port: deploymentPort() || 80,
		};

		const response = await httpRequest<UpdateManagedURLResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/infrastructure/managed-url/${props.managedUrl.id}`,
			{
				method: "PATCH",
				body: JSON.stringify(requestBody),
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);

		if (!response.ok) {
			console.error("Failed to update managed URL:", response.data.error);
			toast("Failed to update managed URL", "error");
			return;
		}

		toast("Managed URL updated successfully", "success");
		props.onUpdate?.();
	};

	const urlInput = () => {
		const urlTypeVal = urlType();
		switch (urlTypeVal) {
			case "proxyDeployment":
				return (
					<DeploymentOption
						deployment={target()}
						onSelectDeployment={(value) => setTarget(value)}
						port={deploymentPort() || 80}
						onPortChange={(port) => setDeploymentPort(port)}
					/>
				);
			default:
				return <Input disabled={true} placeholder="Select URL Type" class="flex-4" />;
		}
	};

	return (
		<form class="w-full mb-2 p-lg bg-secondary-light rounded-xs" onSubmit={onSubmit}>
			<div class="mb-3 w-full flex items-center justify-between">
				<h1 class="text-lg">Update Managed URL</h1>

				<button
					onClick={(e) => {
						e.stopPropagation();
						props.close();
					}}
					class="text-primary text-sm hover:underline cursor-pointer"
				>
					<FiX size={18} />
				</button>
			</div>
			<div class="flex flex-col items-start justify-center gap-2 w-full">
				<div class="flex items-center justify-center gap-3 w-full">
					<Input
						disabled={true}
						value={props.managedUrl.subDomain}
						styleVariant="medium"
						class="flex-2"
						placeholder="Sub-domain"
					/>
					<span class="h-full">.</span>
					<Input disabled={true} value={props.domainInfo?.name} class="flex-2" placeholder="Domain" />
					<span>/</span>
					<Input
						styleVariant="medium"
						onInput={(e) => setPath(e.currentTarget.value)}
						value={path()}
						class="flex-2"
						placeholder="Path"
					/>
				</div>
				<p class="mx-2">Will point to</p>
				<div class="flex items-center justify-center gap-2 w-full">
					<InputDropdown
						onSelect={(value) => setUrlType(value as urlTypeT)}
						value={urlType() || undefined}
						options={[
							{
								label: "Deployments",
								value: "proxyDeployment",
							},
						]}
						class="flex-2 m-0"
						styleVariant="medium"
						placeholder="Type"
					/>
					<div class="flex-10">{urlInput()}</div>
				</div>

				<div class="w-full flex justify-end mt-4">
					<Button variant={ButtonVariant.Contained}>Update</Button>
				</div>
			</div>
		</form>
	);
};

export { ManagedUrlComponent };
export default ManageUrlRow;
