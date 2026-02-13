import { Show } from "solid-js";
import { FiCopy } from "solid-icons/fi";
import { GetContainerRepositoryInfoResponse } from "~/bindings";
import { useToast } from "~/components";

interface GeneralInfoProps {
    repositoryInfo: GetContainerRepositoryInfoResponse | undefined;
}

const GeneralInfo = (props: GeneralInfoProps) => {
    const toast = useToast();

    const handleCopy = async (text: string) => {
        try {
            await navigator.clipboard.writeText(text);
            toast("Copied to clipboard", "success");
        } catch (error) {
            console.error("Failed to copy:", error);
            toast("Failed to copy", "error");
        }
    };

    return (
        <div class="w-full">
            <Show when={props.repositoryInfo} fallback={<div class="text-gray-400 p-6">Loading...</div>}>
                <div class="p-6 space-y-6">
                    <div>
                        <h2 class="text-white text-xl font-medium mb-4">Repository Details</h2>
                        <div class="space-y-4">
                            {/* Repo Name */}
                            <div class="flex items-center gap-4">
                                <div class="text-gray-400 w-32">Repo Name</div>
                                <div class="flex-1 bg-secondary-dark p-3 rounded flex items-center justify-between">
                                    <span class="text-white">{props.repositoryInfo?.repository?.name}</span>
                                    <button
                                        onClick={() => handleCopy(props.repositoryInfo?.repository?.name || "")}
                                        class="text-gray-400 hover:text-white"
                                        title="Copy"
                                    >
                                        <FiCopy size={16} />
                                    </button>
                                </div>
                            </div>

                            {/* Size */}
                            <div class="flex items-center gap-4">
                                <div class="text-gray-400 w-32">Size</div>
                                <div class="flex-1 bg-secondary-dark p-3 rounded">
                                    <span class="text-white">{Number(props.repositoryInfo?.repository?.size || 0)}</span>
                                </div>
                            </div>

                            {/* Last Updated */}
                            <div class="flex items-center gap-4">
                                <div class="text-gray-400 w-32">Last Updated</div>
                                <div class="flex-1 bg-secondary-dark p-3 rounded">
                                    <span class="text-white">
                                        {props.repositoryInfo?.repository?.lastUpdated
                                            ? new Date(props.repositoryInfo.repository.lastUpdated).toLocaleString("en-US", {
                                                year: "numeric",
                                                month: "short",
                                                day: "numeric",
                                                hour: "2-digit",
                                                minute: "2-digit",
                                                second: "2-digit",
                                            })
                                            : "N/A"}
                                    </span>
                                </div>
                            </div>

                            {/* Created */}
                            <div class="flex items-center gap-4">
                                <div class="text-gray-400 w-32">Created</div>
                                <div class="flex-1 bg-secondary-dark p-3 rounded">
                                    <span class="text-white">
                                        {props.repositoryInfo?.repository?.created
                                            ? new Date(props.repositoryInfo.repository.created).toLocaleString("en-US", {
                                                year: "numeric",
                                                month: "short",
                                                day: "numeric",
                                                hour: "2-digit",
                                                minute: "2-digit",
                                                second: "2-digit",
                                            })
                                            : "N/A"}
                                    </span>
                                </div>
                            </div>
                        </div>
                    </div>
                </div>
            </Show>
        </div>
    );
};

export default GeneralInfo;