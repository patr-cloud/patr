import { useParams } from "@solidjs/router";
import { createMemo, For, Show } from "solid-js";
import { FiCopy, FiMoreVertical, FiTrash2 } from "solid-icons/fi";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { useToast } from "~/components";
import { formatRelativeTime } from "~/utils/func";
import { ListContainerRepositoryTagsResponse } from "~/bindings";

interface ImageRow {
	digest: string;
	tags: string[];
	size?: number;
	lastPushed: Date;
}

interface ContainerImagesProps {
	imageTags: ListContainerRepositoryTagsResponse;
}

const Images = (props: ContainerImagesProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const params = useParams();

	const groupedImages = createMemo(() => {
		if (!props.imageTags) return [];

		// Group tags by digest
		const tagsByDigest = new Map<string, ImageRow>();
		for (const tag of props.imageTags.tags) {
			const existing = tagsByDigest.get(tag.digest);
			if (existing) {
				existing.tags.push(tag.tag);
				// Keep the most recent lastPushed date
				if (new Date(tag.lastUpdated) > existing.lastPushed) {
					existing.lastPushed = new Date(tag.lastUpdated);
				}
			} else {
				tagsByDigest.set(tag.digest, {
					digest: tag.digest,
					tags: [tag.tag],
					lastPushed: new Date(tag.lastUpdated),
				});
			}
		}

		return Array.from(tagsByDigest.values());
	});

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
		<div class="w-full p-6">
			<h2 class="text-white text-xl font-medium mb-4">Images</h2>
			<Show
				when={groupedImages() && groupedImages().length > 0}
				fallback={
					<div class="w-full text-center py-16">
						<p class="text-white text-lg">No Images Found</p>
						<p class="text-gray-400 text-sm mt-2">Push an image to this repository to get started</p>
					</div>
				}
			>
				<div class="w-full overflow-x-auto">
					<table class="w-full">
						<thead>
							<tr class="border-b border-gray-700">
								<th class="text-left text-gray-400 font-medium py-3 px-4">Digest</th>
								<th class="text-left text-gray-400 font-medium py-3 px-4">Tags</th>
								<th class="text-left text-gray-400 font-medium py-3 px-4">Size</th>
								<th class="text-left text-gray-400 font-medium py-3 px-4">Last Pushed</th>
								<th class="text-right text-gray-400 font-medium py-3 px-4">Actions</th>
							</tr>
						</thead>
						<tbody>
							<For each={groupedImages()}>
								{(image) => (
									<tr class="border-b border-gray-700/50 hover:bg-secondary-dark/50">
										<td class="py-3 px-4">
											<div class="flex items-center gap-2">
												<span class="text-white font-mono text-sm truncate max-w-xs" title={image.digest}>
													{image.digest.substring(0, 19)}...
												</span>
												<button
													onClick={() => handleCopy(image.digest)}
													class="text-gray-400 hover:text-white shrink-0"
													title="Copy full digest"
												>
													<FiCopy size={14} />
												</button>
											</div>
										</td>
										<td class="py-3 px-4">
											<div class="flex flex-wrap gap-2">
												<For each={image.tags}>
													{(tag) => (
														<span class="text-primary bg-primary/10 px-2 py-1 rounded text-sm font-mono">{tag}</span>
													)}
												</For>
											</div>
										</td>
										<td class="py-3 px-4">
											<span class="text-gray-400">-</span>
										</td>
										<td class="py-3 px-4">
											<span class="text-white" title={image.lastPushed.toISOString()}>
												{formatRelativeTime(image.lastPushed)}
											</span>
										</td>
										<td class="py-3 px-4">
											<div class="flex items-center justify-end gap-2">
												<button
													onClick={() => handleDelete(image.digest)}
													class="text-gray-400 hover:text-red-500 p-2"
													title="Delete image"
												>
													<FiTrash2 size={16} />
												</button>
												<button class="text-gray-400 hover:text-white p-2" title="More options">
													<FiMoreVertical size={16} />
												</button>
											</div>
										</td>
									</tr>
								)}
							</For>
						</tbody>
					</table>
				</div>
			</Show>
		</div>
	);
};

export default Images;
