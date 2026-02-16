import { useParams } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import { FiCopy, FiTrash2 } from "solid-icons/fi";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { useToast, Table, Link } from "~/components";
import { formatRelativeTime } from "~/utils/func";
import { ListContainerRepositoryTagsResponse } from "~/bindings";

interface ContainerImagesProps {
	imageTags: ListContainerRepositoryTagsResponse;
}

const Images = (props: ContainerImagesProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const params = useParams();

	const [deleteSelected, setDeleteSelected] = createSignal(false);

	const handleCopy = async (text: string) => {
		try {
			await navigator.clipboard.writeText(text);
			toast("Copied to clipboard", "success");
		} catch (error) {
			console.error("Failed to copy:", error);
			toast("Failed to copy", "error");
		}
	};

	const handleDelete = async (digest: string) => {
		const auth = authState();
		const wsId = workspaceId();
		const repoId = params.id;

		if (!auth || auth.type !== "LoggedIn" || !wsId || !repoId) {
			toast("User not logged in", "error");
			return;
		}

		const response = await httpRequest(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}/image/${digest}`,
			{
				method: "DELETE",
				headers: {
					"Content-Type": "application/json",
					Authorization: `Bearer ${auth.accessToken}`,
				},
			}
		);

		if (!response.ok) {
			console.error("Failed to delete image:", response.data.error);
			toast("Failed to delete image", "error");
			return;
		}

		toast("Image deleted successfully", "success");
		// Refetch the list
	};

	return (
		<div class="w-full">
			<Show
				when={props.imageTags && props.imageTags.tags && props.imageTags.tags.length > 0}
				fallback={
					<div class="w-full text-center py-16">
						<p class="text-white text-lg">No Images Found</p>
						<p class="text-gray-400 text-sm mt-2">
							<Link href={`/container-repositories/${params.id}?tab=`} external={false} class="inline">
								Push an image
							</Link>{" "}
							to this repository to get started
						</p>
					</div>
				}
			>
				<Table
					column_grids={["flex-4", "flex-4", "flex-4", "flex-4"]}
					headings={["Tag", "Digest", "Last Pushed", "Actions"]}
					rows={props.imageTags.tags}
					renderRow={(image) => (
						<tr class="table-row">
							<td class="flex-4 flex items-center gap-2">
								<span class="truncate text-gray-300 font-mono text-sm">{image.tag}</span>
							</td>
							<td class="flex-4 flex items-center gap-2">
								<span class="truncate text-gray-300 font-mono text-sm">{image.digest}</span>
								<button
									onClick={() => handleCopy(image.digest)}
									class="text-gray-400 hover:text-white shrink-0"
									title="Copy digest"
								>
									<FiCopy size={14} />
								</button>
							</td>
							<td class="flex-4 text-gray-400 text-sm">{formatRelativeTime(image.lastUpdated)}</td>
							<td class="flex-4 flex items-center justify-center">
								{deleteSelected() ? (
									<div class="flex flex-row items-center gap-2">
										<button
											onClick={() => handleDelete(image.digest)}
											class="text-red-500 transition-colors mr-2"
											title="Confirm delete"
										>
											DELETE
										</button>
										<button
											onClick={() => setDeleteSelected(false)}
											class="text-gray-400 transition-colors"
											title="Cancel delete"
										>
											CANCEL
										</button>
									</div>
								) : (
									<button
										onClick={() => {
											setDeleteSelected(true);
										}}
										class="text-red-500 transition-colors"
										title="Delete image"
									>
										<FiTrash2 size={16} />
									</button>
								)}
							</td>
						</tr>
					)}
				/>
			</Show>
		</div>
	);
};

export default Images;
