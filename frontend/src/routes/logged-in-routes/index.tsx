import { Route, Navigate, useNavigate } from "@solidjs/router";
import { createSignal, onMount, Show, createEffect } from "solid-js";
import CreateWorkspace from "./create-workspace";
import Deployments from "./deployments";
import { LocalStorage } from "~/utils/storage";
import { useAuthState } from "~/utils/state";
import LoadingSpinner from "~/components/loading-spinner";
import AppLayout from "../../pages/app-layout";

// Component to handle initial routing logic for authenticated users
const WorkspaceRouter = () => {
  const [authState] = useAuthState();
  const navigate = useNavigate();
  const [isLoading, setIsLoading] = createSignal(true);
  const [hasWorkspace, setHasWorkspace] = createSignal(false);

  // Check authentication and workspace state
  const checkAuthAndWorkspace = () => {
    // First check authentication
    if (authState().type !== "LoggedIn") {
      setIsLoading(false);
      return;
    }

    // Check if user has any workspaces
    const currentWorkspaceId = LocalStorage.getCurrentWorkspaceId();
    const workspaceIds = LocalStorage.getWorkspaceIds();
    
    // User has workspace if they have a current workspace or any workspace IDs
    const userHasWorkspace = !!currentWorkspaceId || workspaceIds.length > 0;
    setHasWorkspace(userHasWorkspace);
    setIsLoading(false);
  };

  onMount(() => {
    checkAuthAndWorkspace();
  });

  // React to auth state changes
  createEffect(() => {
    if (authState().type !== "LoggedIn") {
      navigate("/login", { replace: true });
    }
  });

  return (
    <Show
      when={!isLoading()}
      fallback={
        <div class="min-h-screen w-full bg-secondary flex items-center justify-center">
          <div class="flex flex-col items-center space-y-4">
            <LoadingSpinner size="lg" />
            <div class="text-white">Loading workspace...</div>
          </div>
        </div>
      }
    >
      <Show
        when={authState().type === "LoggedIn"}
        fallback={<Navigate href="/login" />}
      >
        <Show
          when={hasWorkspace()}
          fallback={<Navigate href="/create-workspace" />}
        >
          <Navigate href="/deployments" />
        </Show>
      </Show>
    </Show>
  );
};

// Route protection component for authenticated routes
const ProtectedRoute = (props: { children: any }) => {
  const [authState] = useAuthState();
  const navigate = useNavigate();

  createEffect(() => {
    if (authState().type !== "LoggedIn") {
      navigate("/login", { replace: true });
    }
  });

  return (
    <Show
      when={authState().type === "LoggedIn"}
      fallback={<Navigate href="/login" />}
    >
      {props.children}
    </Show>
  );
};

// Workspace-required route protection
const WorkspaceProtectedRoute = (props: { children: any }) => {
  const [authState] = useAuthState();
  const navigate = useNavigate();
  const [isLoading, setIsLoading] = createSignal(true);
  const [hasWorkspace, setHasWorkspace] = createSignal(false);

  const checkWorkspaceAccess = () => {
    // First check authentication
    if (authState().type !== "LoggedIn") {
      navigate("/login", { replace: true });
      return;
    }

    // Check workspace access
    const currentWorkspaceId = LocalStorage.getCurrentWorkspaceId();
    const workspaceIds = LocalStorage.getWorkspaceIds();
    
    const userHasWorkspace = !!currentWorkspaceId || workspaceIds.length > 0;
    setHasWorkspace(userHasWorkspace);
    setIsLoading(false);

    // Redirect to create workspace if no workspace exists
    if (!userHasWorkspace) {
      navigate("/create-workspace", { replace: true });
    }
  };

  onMount(() => {
    checkWorkspaceAccess();
  });

  // React to auth state changes
  createEffect(() => {
    if (authState().type !== "LoggedIn") {
      navigate("/login", { replace: true });
    }
  });

  return (
    <Show
      when={!isLoading()}
      fallback={
        <div class="min-h-screen w-full bg-secondary flex items-center justify-center">
          <div class="flex flex-col items-center space-y-4">
            <LoadingSpinner size="lg" />
            <div class="text-white">Checking workspace access...</div>
          </div>
        </div>
      }
    >
      <Show
        when={authState().type === "LoggedIn" && hasWorkspace()}
        fallback={<Navigate href="/create-workspace" />}
      >
        {props.children}
      </Show>
    </Show>
  );
};

export default function LoggedInRoutes() {
  return (
    <>
      {/* Root route - handles initial routing logic */}
      <Route path="/" component={WorkspaceRouter} />
      
      {/* Create workspace route - requires authentication only */}
      <Route 
        path="/create-workspace" 
        component={() => (
          <ProtectedRoute>
            <AppLayout>
              <CreateWorkspace />
            </AppLayout>
          </ProtectedRoute>
        )} 
      />
      
      {/* Deployments route - requires authentication and workspace */}
      <Route 
        path="/deployments" 
        component={() => (
          <WorkspaceProtectedRoute>
            <AppLayout>
              <Deployments />
            </AppLayout>
          </WorkspaceProtectedRoute>
        )} 
      />
    </>
  );
}
