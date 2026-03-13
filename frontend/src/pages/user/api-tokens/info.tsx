import { useNavigate, useParams } from "@solidjs/router";
import { createMemo, createResource, createSignal, Suspense } from "solid-js";
import { GetApiTokenInfoResponse } from "~/bindings";
import {
	DeleteModal,
	Input,
	InputLabel,
	InputType,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import RegenerateModal from "./regenerate-modal";
import { RegenerateApiTokenResponse } from "~/bindings/RegenerateApiTokenResponse";
import ApiTokenModal from "./api-token-modal";

const ApiTokenInfo = () => {
	const [authState] = useAuthState();
	const toast = useToast();
	const navigate = useNavigate();
	const params = useParams();
	const [isDeleteModalOpen, setIsDeleteModalOpen] = createSignal(false);
	const [isRegenerateModalOpen, setIsRegenerateModalOpen] = createSignal(false);
	const [isApiTokenModalOpen, setIsApiTokenModalOpen] = createSignal(false);
	const [newApiToken, setNewApiToken] = createSignal<string>("");
	if (!params.id) {
		return <div>Invalid API Token ID</div>;
	}

	const fetchParams = createMemo(() => {
		return [authState()] as const;
	});

	const [apiTokenInfo] = createResource(fetchParams, async ([auth]) => {
		if (!auth || auth.type !== "LoggedIn") {
			return undefined;
		}

		const response = await httpRequest<GetApiTokenInfoResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/api-token/${params.id}`,
			{
				method: "GET",
			}
		);

		if (!response.ok) {
			console.error("Failed to fetch API Token Info:", response.data.error);
			toast("Failed to fetch API Token Info", "error");
			return undefined;
		}

		return { ...response.data };
	});

	const onClickDelete = async (e: EventT<MouseEvent, HTMLButtonElement>) => {
		e.preventDefault();

		const auth = authState();

		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to delete an API Token", "error");
			return;
		}

		const response = await httpRequest<void>(`${import.meta.env.VITE_BASE_URL}/api/user/api-token/${params.id}`, {
			method: "DELETE",
		});

		if (!response.ok) {
			console.error("Failed to delete API Token:", response.data.error);
			toast("Failed to delete API Token", "error");
			return;
		}

		toast("API Token deleted successfully", "success");
		navigate("/profile/api-tokens");
	};

	const onClickRegenerate = async (e: EventT<MouseEvent, HTMLButtonElement>) => {
		e.preventDefault();

		const auth = authState();

		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to regenerate an API Token", "error");
			return;
		}

		const response = await httpRequest<RegenerateApiTokenResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/user/api-token/${params.id}/regenerate`,
			{
				method: "POST",
			}
		);

		if (!response.ok) {
			console.error("Failed to regenerate API Token:", response.data.error);
			toast("Failed to regenerate API Token", "error");
			return;
		}

		toast("API Token regenerated successfully", "success");
		setNewApiToken(response.data.token);
		setIsRegenerateModalOpen(false);
		setIsApiTokenModalOpen(true);
	};

	return (
		<PageContainer>
			<Suspense fallback={<div>Loading API Token Info...</div>}>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "API Tokens",
							url: "/profile/api-tokens",
						},
						{
							label: apiTokenInfo()?.name || "",
						},
					]}
					subText="Manage API Token here"
					actions={() => (
						<div class="flex gap-2 px-2">
							<RegenerateModal
								title="Regenerate API Token"
								onClickRegenerate={onClickRegenerate}
								resourceName={apiTokenInfo()?.name || ""}
								isOpen={isRegenerateModalOpen}
								setIsOpen={setIsRegenerateModalOpen}
							/>
							<DeleteModal
								title="Delete API Token"
								onClickDelete={onClickDelete}
								resourceName={apiTokenInfo()?.name || ""}
								isOpen={isDeleteModalOpen}
								setIsOpen={setIsDeleteModalOpen}
							/>
						</div>
					)}
				/>
				<PageContainerBody class="flex flex-col gap-8">
					<div class="flex flex-col gap-4 items-start w-full">
						<div class="flex gap-8 items-center w-full">
							<InputLabel parentClass="flex-2" for="deployment-id" label="ID" />
							<Input
								value={apiTokenInfo()?.id || ""}
								disabled={true}
								class="flex-10"
								name="deployment-id"
								placeholder="Deployment ID"
								type={InputType.Text}
							/>
						</div>

						<div class="flex gap-8 items-center w-full">
							<InputLabel parentClass="flex-2" for="deployment-name" label="Name" />
							<Input
								value={apiTokenInfo()?.name || ""}
								class="flex-10"
								name="deployment-name"
								placeholder="Deployment Name"
								type={InputType.Text}
								disabled={true}
							/>
						</div>
					</div>
				</PageContainerBody>
			</Suspense>

			<ApiTokenModal
				isOpen={isApiTokenModalOpen}
				setIsOpen={setIsApiTokenModalOpen}
				token={newApiToken}
				onClose={() => navigate("/profile/api-tokens")}
			/>
		</PageContainer>
	);
};

export default ApiTokenInfo;
