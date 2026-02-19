import { createMemo, createResource, createSignal } from "solid-js";
import {
	Input,
	InputLabel,
	InputType,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	InputDropdown,
	ButtonVariant,
	Button,
} from "~/components";
import EnvInput from "./env-input";
import { FiChevronDown } from "solid-icons/fi";
import {
	Base64String,
	CreateDeploymentRequest,
	CreateDeploymentResponse,
	DeploymentProbe,
	EnvironmentVariableValue,
	ExposedPortType,
	ListRunnersForWorkspaceResponse,
} from "~/bindings";
import PortInput from "./port";
import { createFormAction, useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { convertFileToBase64, Uuid } from "~/utils/func";
import { useToast } from "~/components";
import ProbeInput from "~/pages/deployment/probe-input";
import ConfigMount, { ConfigMountT } from "~/pages/deployment/config-mount";
import { useNavigate } from "@solidjs/router";

const CreateDeploymentPage = () => {
	const [authState] = useAuthState();
	const [lastUsedWorkspaceId] = useLastWorkspaceId();
	const toast = useToast();

	const fetchParams = createMemo(() => {
		return [authState(), lastUsedWorkspaceId()] as const;
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

	const navigate = useNavigate();
	const [name, setName] = createSignal<string>("");
	const [runner, setRunner] = createSignal<string>("");
	const [imageName, setImageName] = createSignal<string>("");
	const [imageTag, setImageTag] = createSignal<string>("");
	const [configFiles, setConfigFiles] = createSignal<ConfigMountT>({});
	const [startupProbe, setStartupProbe] = createSignal<DeploymentProbe | undefined>(undefined);

	const [registry, setRegistry] = createSignal<string>("");
	const [envList, setEnvList] = createSignal<{ key: string; value: EnvironmentVariableValue }[]>([]);

	const [portList, setPortList] = createSignal<{
		[key: string]: ExposedPortType;
	}>({});

	const { onSubmit, isLoading } = createFormAction(async ({ accessToken, workspaceId }) => {
		let configMounts: Record<string, Base64String> = {};
		for (const [key, file] of Object.entries(configFiles())) {
			const byteArray = await convertFileToBase64(file);
			configMounts[key] = byteArray;
		}

		const requestBody: CreateDeploymentRequest = {
			name: name(),
			imageName: imageName(),
			imageTag: imageTag(),
			registry: registry(),
			runner: runner(),
			machineType: Uuid("b3cf3771-fa39-4281-bfdf-eb2e65a061b6"),
			minHorizontalScale: 1,
			maxHorizontalScale: 2,
			environmentVariables: Object.fromEntries(envList().map((env) => [env.key, env.value])),
			ports: portList(),
			deployOnCreate: false,
			deployOnPush: false,
			configMounts,
		};

		console.log(requestBody);

		const response = await httpRequest<CreateDeploymentResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/deployment`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			console.error("Failed to create deployment:", response.data.error);
			toast("Failed to create deployment", "error");
			return;
		}

		toast("Deployment created successfully", "success");

		navigate(`/deployments/${response.data.id}`);
		console.log("Deployment created:", response.data);
	});

	return (
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Deployments",
						url: "/deployments",
					},
					{
						label: "New",
					},
				]}
				subText="A deployment represents a containerized application running on a runner."
			/>
			<PageContainerBody class="flex flex-col justify-between gap-8">
				<form onSubmit={onSubmit} class="flex flex-col gap-6 justify-between w-full flex-1">
					<div class="flex flex-col gap-5  items-start w-full">
						<div class="flex gap-8 items-center w-full">
							<InputLabel parentClass="flex-2" for="deployment-name" label="Name" />
							<Input
								value={name()}
								onInput={(e) => {
									e.preventDefault();
									setName(e.currentTarget.value);
								}}
								class="flex-10"
								name="deployment-name"
								placeholder="Enter Deployment Name"
								type={InputType.Text}
							/>
						</div>

						<div class="flex gap-8 items-center w-full">
							<InputLabel parentClass="flex-2" for="deployment-registry" label="Image" />
							<div class="flex-10 flex items-center gap-4 w-full">
								<InputDropdown
									options={[
										{ value: "registry.patr.cloud", label: "Patr Registry" },
										{ value: "docker.io", label: "Docker Hub" },
									]}
									endIcon={() => (
										<button>
											<FiChevronDown size={16} />
										</button>
									)}
									value={registry()}
									onSelect={setRegistry}
									class="flex-4"
									name="deployment-registry"
									placeholder="Select Registry"
								/>

								<Input
									class="flex-6"
									placeholder="Image Name"
									type={InputType.Text}
									value={imageName()}
									onInput={(e) => setImageName(e.currentTarget.value)}
								/>

								<Input
									class="flex-2"
									placeholder="Image Tag"
									type={InputType.Text}
									value={imageTag()}
									onInput={(e) => setImageTag(e.currentTarget.value)}
								/>
							</div>
						</div>

						<div class="flex gap-8 items-center w-full">
							<InputLabel parentClass="flex-2" for="deployment-runner" label="Runner" />
							<div class="flex-10 flex items-center gap-4 w-full">
								<InputDropdown
									options={
										runners()?.runners.map((runner) => ({
											value: runner.id,
											label: runner.name,
										})) ?? []
									}
									endIcon={() => (
										<button>
											<FiChevronDown size={16} />
										</button>
									)}
									value={runner()}
									onSelect={setRunner}
									class="flex-4"
									name="deployment-runner"
									placeholder="Select Runner"
								/>
							</div>
						</div>

						<EnvInput
							envList={envList}
							onAdd={(key, value) => {
								setEnvList((prev) => [...prev, { key, value }]);
							}}
							onDelete={(key) => {
								setEnvList((prev) => prev.filter((env) => env.key !== key));
							}}
						/>

						<PortInput
							onAdd={(key, value) => {
								setPortList((prev) => ({ ...prev, [key]: value }));
							}}
							onDelete={(key) => {
								setPortList((prev) => {
									const newPorts = { ...prev };
									delete newPorts[key];
									return newPorts;
								});
							}}
							portList={portList}
						/>

						<ProbeInput
							probe={[startupProbe, setStartupProbe]}
							ports={Object.keys(portList()).map((port) => parseInt(port))}
						/>

						<ConfigMount selectedFiles={configFiles} setSelectedFiles={setConfigFiles} />
					</div>

					<div class="w-full flex items-end justify-end">
						<Button
							loading={isLoading}
							loadingContent={() => <span>Creating Deployment...</span>}
							type="submit"
							variant={ButtonVariant.Contained}
						>
							Create
						</Button>
					</div>
				</form>
			</PageContainerBody>
		</PageContainer>
	);
};

export default CreateDeploymentPage;
