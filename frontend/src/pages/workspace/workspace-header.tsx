import { A, useNavigate, useParams, useLocation } from "@solidjs/router";
import { Button, ButtonVariant, Link, PageContainerHead } from "~/components";

interface WorkspaceHeaderProps {
  workspaceName?: string;
  activeTab: "workspace" | "roles";
}

const WorkspaceHeader = (props: WorkspaceHeaderProps) => {
  const navigate = useNavigate();
  const params = useParams();
  const location = useLocation();

  return (
    <PageContainerHead
      title="Manage Workspace"
      titleUrl="/workspaces"
      subTitle={props.workspaceName || "Loading..."}
      actions={() => (
        props.activeTab === "roles" && !location.pathname.includes("/new") && (
          <Link
            href={`/workspaces/${params.id}/roles/new`}
            buttonVariant={ButtonVariant.Contained}
            external={false}
          >
            Create New Role
          </Link>
        )

      )}
      bottomContent={() => (
        <div class="w-full text-white flex gap-4">
          <A
            href={`/workspaces/${params.id}`}
            class={`pb-2 px-2 border-b-2 ${props.activeTab === "workspace"
              ? "border-primary"
              : "border-transparent"
              }`}
          >
            Manage Workspace
          </A>

          <A
            href={`/workspaces/${params.id}/roles`}
            onClick={() => navigate(`/workspaces/${params.id}/roles`)}
            class={`pb-2 px-2 border-b-2 ${props.activeTab === "roles"
              ? "border-primary"
              : "border-transparent"
              }`}
          >
            Manage Roles
          </A>
        </div>
      )}
    />
  );
};

export default WorkspaceHeader;
