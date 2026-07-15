import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, ErrorBoundary } from "solid-js";
import {
	Button,
	ButtonVariant,
	Modal,
	ModalContainer,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import { useContainerRegistryInfoQuery } from "~/hooks/fetch";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { Color } from "~/utils/color";
import { httpRequest } from "~/utils/http-request";
import { shortDigest } from "./-components/registry-ui";
import ManifestDetail from "./-components/manifest-detail";

const ContainerManifestDetail = () => {
	const [workspaceId] = useLastWorkspaceId();
	const [authState] = useAuthState();
	const toast = useToast();
	const navigate = useNavigate();
	const params = Route.useParams();

	const repoInfoQuery = useContainerRegistryInfoQuery(() => params().id);

	const repoName = () => repoInfoQuery.data?.repository?.name;
	const imagePath = () => `registry.patr.cloud/${workspaceId() ?? ""}/${repoName() ?? ""}`;

	const [deleting, setDeleting] = createSignal(false);

	const handleDelete = async (setClose: (open: boolean) => void) => {
		const auth = authState();
		const wsId = workspaceId();
		if (!auth || auth.type !== "LoggedIn" || !wsId) {
			toast("User not logged in", "error");
			return;
		}
		setDeleting(true);
		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${params().id}/manifest/${params().digest}`,
			{ method: "DELETE" }
		);
		setDeleting(false);
		if (!response.ok) {
			toast("Failed to delete image", "error");
			return;
		}
		toast("Image deleted successfully", "success");
		setClose(false);
		navigate({
			to: "/container-registry/$id",
			params: { id: params().id },
			search: { tab: "tags" },
		});
	};

	return (
		<>
			<Title>Image Details | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{ label: "Container Repositories", url: "/container-registry" },
						{
							label: repoName() || "Loading...",
							url: `/container-registry/${params().id}?tab=tags`,
						},
						{ label: shortDigest(params().digest) },
					]}
					subText="Details for this exact image"
					class="justify-between items-center"
					actions={() => (
						<Modal
							renderTrigger={(setOpen) => (
								<Button
									color={Color.Error}
									variant={ButtonVariant.Outlined}
									onClick={() => setOpen(true)}
								>
									Delete
								</Button>
							)}
							renderModalContent={(setClose) => (
								<ModalContainer closeFn={setClose} width="28rem">
									<h3 class="text-white text-lg font-medium mb-2">Delete this image?</h3>
									<p class="text-gray-400 text-sm mb-6">
										This permanently removes{" "}
										<span class="font-mono text-gray-300">{shortDigest(params().digest)}</span> from{" "}
										{repoName() || "this repository"}. This can't be undone.
									</p>
									<div class="flex items-center justify-end gap-3">
										<Button variant={ButtonVariant.Plain} onClick={() => setClose(false)}>
											Cancel
										</Button>
										<Button
											color={Color.Error}
											variant={ButtonVariant.Contained}
											loading={deleting()}
											onClick={() => handleDelete(setClose)}
										>
											Delete
										</Button>
									</div>
								</ModalContainer>
							)}
						/>
					)}
				/>

				<PageContainerBody class="flex flex-col gap-8">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading image: {err.message}</p>
								<Button variant={ButtonVariant.Outlined} onClick={reset}>
									Retry
								</Button>
							</div>
						)}
					>
						<ManifestDetail
							repoId={() => params().id}
							reference={() => params().digest}
							imagePath={imagePath}
						/>
					</ErrorBoundary>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/container-registry/$id_/manifest/$digest")({
	component: ContainerManifestDetail,
});
