import { useParams, useSearchParams } from "@solidjs/router";
import { createMemo, createResource, ErrorBoundary, Suspense } from "solid-js";
import { HeadTab, PageContainer, PageContainerBody, PageContainerHead, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { GetContainerRepositoryInfoResponse, ListContainerRepositoryTagsResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import GeneralInfo from "./general-info";
import ContainerImages from "./container-images";

const ContainerInfo = () => {
    const [authState] = useAuthState();
    const [workspaceId] = useLastWorkspaceId();
    const toast = useToast();
    const params = useParams();

    const [searchParams, setSearchParams] = useSearchParams();
    const tab = () => (searchParams.tab as string) || "";

    const resourceParams = createMemo(() => {
        return [authState(), workspaceId(), params.id] as const;
    });

    const [repositoryInfo] = createResource(resourceParams, async ([auth, wsId, repoId]) => {
        if (!wsId || !auth || auth.type !== "LoggedIn" || !repoId) {
            return undefined;
        }

        const response = await httpRequest<GetContainerRepositoryInfoResponse>(
            `${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}`,
            {
                method: "GET",
                headers: {
                    "Content-Type": "application/json",
                    Authorization: `Bearer ${auth.accessToken}`,
                },
            }
        );

        if (!response.ok) {
            console.error("Failed to fetch repository info:", response.data.error);
            toast("Failed to fetch repository info", "error");
            return undefined;
        }

        return response.data;
    });

    const [imageTags] = createResource(resourceParams, async ([auth, wsId, repoId]) => {
        if (!wsId || !auth || auth.type !== "LoggedIn" || !repoId) {
            return undefined;
        }

        const response = await httpRequest<ListContainerRepositoryTagsResponse>(
            `${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/container-registry/${repoId}/tag`,
            {
                method: "GET",
                headers: {
                    "Content-Type": "application/json",
                    Authorization: `Bearer ${auth.accessToken}`,
                },
            }
        );

        if (!response.ok) {
            console.error("Failed to fetch image tags:", response.data.error);
            toast("Failed to fetch image tags", "error");
            return undefined;
        }

        return response.data;
    });

    const renderTab = () => {
        switch (tab()) {
            case "images":
                const tags = imageTags();
                if (!tags) return <div>Loading image tags...</div>;
                return <ContainerImages imageTags={tags} />;
            case "general":
            case "":
                return <GeneralInfo repositoryInfo={repositoryInfo()} />;
            default:
                return <div class="text-white">No such tab</div>;
        }
    };

    return (
        <PageContainer>
            <PageContainerHead
                breadcrumbs={[
                    {
                        label: "Repository",
                        url: "/container-repositories",
                    },
                    {
                        label: repositoryInfo()?.repository?.name || "Loading...",
                    }
                ]}
                subText="View and manage container repository images"
                class="justify-between items-center"
                bottomContent={() => (
                    <HeadTab
                        tab={tab}
                        searchParams={searchParams}
                        setSearchParams={setSearchParams}
                        tabItems={[
                            {
                                label: "General",
                                value: "",
                                onClick: (value) => setSearchParams({ tab: value }),
                            },
                            {
                                label: "Images",
                                value: "images",
                                onClick: (value) => setSearchParams({ tab: value }),
                            },
                        ]}
                    />
                )}
            />
            <PageContainerBody class="flex flex-col justify-between gap-8">
                <ErrorBoundary
                    fallback={(err, reset) => (
                        <div>
                            <p>Error loading repository info: {err.message}</p>
                            <button onClick={reset}>Retry</button>
                        </div>
                    )}
                >
                    <Suspense fallback={<div>Loading repository info...</div>}>{renderTab()}</Suspense>
                </ErrorBoundary>
            </PageContainerBody>
        </PageContainer>
    )
}
export default ContainerInfo;