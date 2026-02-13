import { createMemo, createResource, createSignal, For, Show } from "solid-js";
import { useNavigate } from "@solidjs/router";
import {
    Button,
    ButtonVariant,
    Input,
    InputLabel,
    InputType,
    PageContainer,
    PageContainerBody,
    PageContainerHead,
    Table,
    ToggleSwitch,
    useToast,
} from "~/components";
import { useAuthState, useLastWorkspaceId } from "~/hooks/state-hooks";
import {
    CreateContainerRepositoryRequest,
    CreateContainerRepositoryResponse,
    ListContainerRepositoriesResponse,
    WithId,
    ContainerRepository,
} from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import { formatRelativeTime } from "~/utils/func";

const CreateContainerRepository = () => {
    const [authState] = useAuthState();
    const [workspaceId] = useLastWorkspaceId();
    const toast = useToast();

    const [repositoryName, setRepositoryName] = createSignal("");
    const [isSubmitting, setIsSubmitting] = createSignal(false);

    const resourceParams = createMemo(() => {
        return [authState(), workspaceId()] as const;
    });

    const [repositories, { refetch }] = createResource(resourceParams, async ([auth, wsId]) => {
        if (!wsId || !auth || auth.type !== "LoggedIn") {
            return undefined;
        }

        const response = await httpRequest<ListContainerRepositoriesResponse>(
            `${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry`,
            {
                method: "GET",
                headers: {
                    "Content-Type": "application/json",
                    Authorization: `Bearer ${auth.accessToken}`,
                },
            }
        );

        if (!response.ok) {
            console.error("Failed to fetch repositories:", response.data.error);
            toast("Failed to fetch repositories", "error");
            return undefined;
        }

        return response.data;
    });

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
                    Authorization: `Bearer ${auth.accessToken}`,
                },
                body: JSON.stringify(requestBody),
            }
        );

        setIsSubmitting(false);
        console.log("Create Repository Response:", response);

        if (!response.ok) {
            console.error("Failed to create repository:", response.data.error);
            toast("Failed to create repository", "error");
            return;
        }

        toast("Repository created successfully", "success");
        setRepositoryName("");
        refetch();
    };

    return (
        <PageContainer>
            <PageContainerHead
                breadcrumbs={[
                    {
                        label: "Repository",
                    },
                ]}
                subText="Create Deployments, Databases, Object Storage, Static Sites, Upgrade Paths and manage Repositories"
            />

            <PageContainerBody>
                <div class="w-full flex flex-col gap-6">
                    {/* Create Repository Form */}
                    <div class="bg-secondary-light rounded-lg p-6">
                        <h2 class="text-white text-lg font-medium mb-6">Create New Repository</h2>

                        <form onSubmit={handleSubmit} class="flex flex-col gap-4">
                            <div class="flex flex-row gap-4 items-center">
                                <InputLabel for="repository-name" label="Repository Name:" parentClass="w-1/6" />
                                <Input
                                    class="w-full"
                                    id="repository-name"
                                    type={InputType.Text}
                                    value={repositoryName()}
                                    onInput={(e) => setRepositoryName(e.currentTarget.value)}
                                    placeholder="Enter Name"
                                    disabled={isSubmitting()}
                                />
                            </div>


                            <div class="flex justify-end">
                                <Button type="submit" variant={ButtonVariant.Contained} disabled={isSubmitting()}>
                                    {isSubmitting() ? "CREATING..." : "CREATE REPOSITORY"}
                                </Button>
                            </div>
                        </form>
                    </div>

                    {/* Repository Table */}
                    <div class="w-full">
                        <Show
                            when={repositories()?.repositories && repositories()!.repositories.length > 0}
                            fallback={
                                <div class="w-full text-center py-16">
                                    <p class="text-white text-lg">No Repository Exist</p>
                                </div>
                            }
                        >
                            <Table
                                column_grids={["flex-1", "flex-1", "flex-1"]}
                                headings={["Repository", "Visibility", "Date Created"]}
                                rows={repositories()?.repositories || []}
                                renderRow={(repo: WithId<ContainerRepository>) => (
                                    <tr class="table-row">
                                        <td class="flex-1">
                                            <span class="truncate">{repo.name}</span>
                                        </td>
                                        <td class="flex-1">{formatRelativeTime(repo.created)}</td>
                                    </tr>
                                )}
                            />
                        </Show>
                    </div>
                </div>
            </PageContainerBody>
        </PageContainer>
    );
};

export default CreateContainerRepository;