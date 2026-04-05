import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { FiInfo } from "solid-icons/fi";
import { createSignal, Show } from "solid-js";
import { CreateContainerRepositoryRequest, CreateContainerRepositoryResponse } from "~/bindings";
import {
	Alert,
	Button,
	ButtonVariant,
	CopyableField,
	CopyableFieldVariant,
	Input,
	InputType,
	InputLabel,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import { createFormAction } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

const CreateContainerRepository = () => {
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();

	const [repositoryName, setRepositoryName] = createSignal("");
	const [nameError, setNameError] = createSignal("");

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId: wsId }) => {
		if (!repositoryName().trim()) {
			setNameError("Repository name is required.");
			return;
		}

		const requestBody: CreateContainerRepositoryRequest = {
			name: repositoryName().trim(),
		};

		const response = await httpRequest<CreateContainerRepositoryResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			setNameError("Failed to create repository. Please try a different name.");
			return;
		}

		toast("Repository created successfully", "success");
		navigate({ to: `/container-registry/${response.data.id}` });
	});

	return (
		<>
			<Title>New Container Repository | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Container Repositories",
							url: "/container-registry",
						},
						{
							label: "New",
						},
					]}
					subText="Store and manage container images for your deployments"
				/>
				<PageContainerBody class="flex flex-col">
					<form noValidate onSubmit={onSubmit} class="flex flex-col gap-8 w-full">
						<div class="flex flex-col gap-6 w-full">
							<div class="flex gap-8 items-center w-full">
								<InputLabel parentClass="flex-2" for="repository-name" label="Repository Name" />
								<div class="flex-10 flex flex-col">
									<Input
										value={repositoryName()}
										onInput={(e) => {
											setRepositoryName(e.currentTarget.value);
											setNameError("");
										}}
										id="repository-name"
										name="repository-name"
										placeholder="Enter Repository Name"
										type={InputType.Text}
									/>
									<Show when={nameError()}>
										<div class="mt-1">
											<Alert message={nameError()} type="error" />
										</div>
									</Show>
								</div>
							</div>
							<Show when={repositoryName().trim()}>
								<div class="flex items-start w-full gap-8">
									<div class="flex-2" />
									<div class="flex-10 flex items-center gap-2">
										<FiInfo size={14} class="text-grey shrink-0" aria-hidden="true" />
										<span class="text-grey text-xs">
											Your container repository will be created as:
										</span>
										<CopyableField
											variant={CopyableFieldVariant.Text}
											value={`registry.patr.cloud/${workspaceId()}/${repositoryName().trim()}`}
											innerClass="text-white font-semibold"
										/>
									</div>
								</div>
							</Show>
						</div>

						<div class="w-full flex justify-end">
							<Button
								variant={ButtonVariant.Contained}
								type="submit"
								loading={isLoading}
								loadingContent={() => <span>Creating...</span>}
							>
								Create Repository
							</Button>
						</div>
					</form>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/container-registry/new")({
	component: CreateContainerRepository,
});
