import { useNavigate, useParams } from "@solidjs/router";
import { PageContainerHead } from "~/components";

interface WorkspaceHeaderProps {
    workspaceName?: string;
    activeTab: "workspace" | "roles";
}

const WorkspaceHeader = (props: WorkspaceHeaderProps) => {
    const navigate = useNavigate();
    const params = useParams();

    return (
        <PageContainerHead
            title="Manage Workspace"
            titleUrl="/workspaces"
            subTitle={props.workspaceName || ""}
            bottomContent={() => (
                <div class="w-full text-white flex gap-4">
                    <button
                        onClick={() => navigate(`/workspaces/${params.id}`)}
                        class={`pb-2 px-2 border-b-2 ${props.activeTab === "workspace"
                            ? "border-primary"
                            : "border-transparent"
                            }`}
                    >
                        Manage Workspace
                    </button>
                    <button
                        onClick={() => navigate(`/workspaces/${params.id}/roles`)}
                        class={`pb-2 px-2 border-b-2 ${props.activeTab === "roles"
                            ? "border-primary"
                            : "border-transparent"
                            }`}
                    >
                        Manage Roles
                    </button>
                </div>
            )}
        />
    );
};

export default WorkspaceHeader;
