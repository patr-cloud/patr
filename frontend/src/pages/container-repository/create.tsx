import { useNavigate } from "@solidjs/router";
import { createSignal } from "solid-js";
import { CreateContainerRepositoryRequest, CreateContainerRepositoryResponse } from "~/bindings";
import {
	Button,
	ButtonVariant,
	Input,
	InputLabel,
	InputType,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";

const CreateContainerRepository = () => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const navigate = useNavigate();

	const [repositoryName, setRepositoryName] = createSignal("");
	const [isSubmitting, setIsSubmitting] = createSignal(false);

	const handleSubmit = async (e: Event) => {
		e.preventDefault();

		const auth = authState();
		const wsId = workspaceId();

		if (!auth || auth.type !== "LoggedIn" || !wsId) {
			toast("User not logged in", "error");
			return;
		}

		if (!repositoryName().trim()) {
			toast("Repository name is required", "error");
			return;
		}

		setIsSubmitting(true);

		const requestBody: CreateContainerRepositoryRequest = {
			name: repositoryName().trim(),
		};

		const response = await httpRequest<CreateContainerRepositoryResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry`,
			{
				method: "POST",
				headers: {
					"Content-Type": "application/json",
				},
				body: JSON.stringify(requestBody),
			}
		);

		setIsSubmitting(false);

		if (!response.ok) {
			console.error("Failed to create repository:", response.data.error);
			toast("Failed to create repository", "error");
			return;
		}

		toast("Repository created successfully", "success");
		setRepositoryName("");
		navigate(`/container-repositories/${response.data.id}`);
	};
	return (
		<PageContainer>
			<PageContainerHead
				breadcrumbs={[
					{
						label: "Repositories",
						url: "/container-repositories",
					},
					{
						label: "Add",
					},
				]}
				subText="Create Deployments, Databases, Object Storage, Static Sites, Upgrade Paths and manage Repositories"
			/>
			<PageContainerBody class="flex flex-col justify-between gap-8">
				<form onSubmit={handleSubmit} class="flex flex-col gap-8 items-start w-full justify-between flex-1">
					<div class="flex w-full flex-col justify-between gap-6 h-full flex-1">
						<div class="flex flex-col gap-6 items-start w-full">
							<h1 class="text-md">Create Repository</h1>

							<div class="flex gap-8 items-center w-full">
								<InputLabel parentClass="flex-2" for="repository-name" label="Repository Name" />
								<Input
									value={repositoryName()}
									onInput={(e) => {
										setRepositoryName(e.currentTarget.value);
									}}
									class="flex-10"
									name="repository-name"
									placeholder="Enter Repository Name"
									type={InputType.Text}
								/>
							</div>
						</div>
					</div>

					<div class="w-full flex justify-end">
						<Button variant={ButtonVariant.Contained} type="submit">
							Create Repository
						</Button>
					</div>
				</form>
			</PageContainerBody>
		</PageContainer>
	);
};
export default CreateContainerRepository;
