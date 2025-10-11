import { createSignal, onMount, Show, For, createResource } from "solid-js";
import { useNavigate } from "@solidjs/router";
import { LocalStorage } from "~/utils/storage";
import { useAuthState } from "~/utils/state";
import { api, type Deployment } from "~/utils/api";
import LoadingSpinner from "~/components/loading-spinner";
import ErrorMessage from "~/components/error-message";

const Deployments = () => {
  const navigate = useNavigate();
  const [authState] = useAuthState();
  const [currentWorkspaceId, setCurrentWorkspaceId] = createSignal<string | undefined>();

  // Create resource for fetching deployments
  const [deployments, { refetch: refetchDeployments }] = createResource(
    currentWorkspaceId,
    async (workspaceId: string) => {
      try {
        const response = await api.getDeployments(workspaceId);
        if (!response.success) {
          // Create more specific error messages based on error type
          let errorMessage = response.message;
          switch (response.error) {
            case 'authentication_failed':
              errorMessage = 'Your session has expired. Please log in again.';
              break;
            case 'network_error':
              errorMessage = 'Unable to connect to the server. Please check your internet connection.';
              break;
            case 'not_found':
              errorMessage = 'Workspace not found. Please check if the workspace still exists.';
              break;
            case 'forbidden':
              errorMessage = 'You do not have permission to view deployments in this workspace.';
              break;
            default:
              errorMessage = response.message || 'Failed to load deployments. Please try again.';
          }
          throw new Error(errorMessage);
        }
        return response.deployments;
      } catch (error) {
        // Handle network errors and other exceptions
        if (error instanceof Error) {
          throw error;
        }
        throw new Error('An unexpected error occurred while loading deployments.');
      }
    }
  );

  // Helper function to format dates
  const formatDate = (dateString: string) => {
    const date = new Date(dateString);
    return date.toLocaleDateString('en-US', {
      year: 'numeric',
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit'
    });
  };

  // Helper function to get status styling
  const getStatusStyle = (status: Deployment['status']) => {
    switch (status) {
      case 'running':
        return 'bg-success-light text-success border-success';
      case 'stopped':
        return 'bg-gray-100 text-gray-700 border-gray-300';
      case 'error':
        return 'bg-error-light text-error border-error';
      case 'pending':
        return 'bg-yellow-100 text-yellow-700 border-yellow-300';
      case 'deploying':
        return 'bg-blue-100 text-blue-700 border-blue-300';
      default:
        return 'bg-gray-100 text-gray-700 border-gray-300';
    }
  };

  // Helper function to capitalize status
  const capitalizeStatus = (status: string) => {
    return status.charAt(0).toUpperCase() + status.slice(1);
  };

  onMount(() => {
    // Check authentication
    if (authState().type !== "LoggedIn") {
      navigate("/login", { replace: true });
      return;
    }

    // Check workspace access
    const workspaceId = LocalStorage.getCurrentWorkspaceId();
    const workspaceIds = LocalStorage.getWorkspaceIds();
    
    if (!workspaceId && workspaceIds.length === 0) {
      // No workspace available, redirect to create workspace
      navigate("/create-workspace", { replace: true });
      return;
    }

    // Set current workspace ID for display
    setCurrentWorkspaceId(workspaceId || workspaceIds[0]);
  });

  return (
    <div class="max-w-6xl mx-auto">
        <div class="mb-8">
          <h1 class="text-3xl font-bold text-white mb-2">Deployments</h1>
          <p class="text-gray-400">
            Manage your application deployments and services.
          </p>
          <Show when={currentWorkspaceId()}>
            <p class="text-sm text-gray-500 mt-2">
              Workspace ID: {currentWorkspaceId()}
            </p>
          </Show>
        </div>

        {/* Loading State */}
        <Show when={deployments.loading}>
          <div class="bg-secondary-dark rounded-lg border border-secondary-medium p-8 text-center">
            <LoadingSpinner size="lg" class="mx-auto mb-4" />
            <p class="text-gray-400">Loading deployments...</p>
          </div>
        </Show>

        {/* Error State */}
        <Show when={deployments.error}>
          <div class="mb-6">
            <ErrorMessage
              message={deployments.error?.message || "Failed to load deployments"}
              showRetry={true}
              onRetry={() => {
                console.log('Retrying deployment fetch...');
                refetchDeployments();
              }}
              dismissible={false}
            />
            {/* Additional error context for network issues */}
            <Show when={deployments.error?.message.includes('network') || deployments.error?.message.includes('connect')}>
              <div class="mt-4 p-4 bg-yellow-50 border border-yellow-200 rounded-xs">
                <div class="flex">
                  <div class="flex-shrink-0">
                    <svg class="h-5 w-5 text-yellow-400" viewBox="0 0 20 20" fill="currentColor">
                      <path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
                    </svg>
                  </div>
                  <div class="ml-3">
                    <h3 class="text-sm font-medium text-yellow-800">Connection Issue</h3>
                    <div class="mt-2 text-sm text-yellow-700">
                      <p>If the problem persists, please check:</p>
                      <ul class="list-disc list-inside mt-1">
                        <li>Your internet connection</li>
                        <li>Whether the Patr service is experiencing issues</li>
                        <li>Your firewall or VPN settings</li>
                      </ul>
                    </div>
                  </div>
                </div>
              </div>
            </Show>
          </div>
        </Show>

        {/* Deployments List */}
        <Show when={!deployments.loading && !deployments.error}>
          <Show
            when={deployments() && deployments()!.length > 0}
            fallback={
              <div class="bg-secondary-dark rounded-lg border border-secondary-medium p-12 text-center">
                <div class="mb-6">
                  <svg
                    class="w-20 h-20 mx-auto text-gray-500"
                    fill="none"
                    stroke="currentColor"
                    viewBox="0 0 24 24"
                    xmlns="http://www.w3.org/2000/svg"
                  >
                    <path
                      stroke-linecap="round"
                      stroke-linejoin="round"
                      stroke-width="1"
                      d="M19 11H5m14 0a2 2 0 012 2v6a2 2 0 01-2 2H5a2 2 0 01-2-2v-6a2 2 0 012-2m14 0V9a2 2 0 00-2-2M5 11V9a2 2 0 012-2m0 0V5a2 2 0 012-2h6a2 2 0 012 2v2M7 7h10"
                    />
                  </svg>
                </div>
                <h3 class="text-xl font-semibold text-white mb-3">No Deployments Yet</h3>
                <p class="text-gray-400 mb-6 max-w-md mx-auto">
                  You haven't created any deployments yet. Get started by deploying your first application to the cloud.
                </p>
                <button 
                  class="bg-primary hover:bg-primary-light text-white font-medium py-3 px-6 rounded-xs transition-colors focus:outline-none focus:ring-2 focus:ring-primary focus:ring-offset-2 focus:ring-offset-secondary-dark"
                  onClick={() => {
                    // Future functionality - for now just show a placeholder
                    alert('Create Deployment functionality will be implemented in a future update.');
                  }}
                >
                  Create Deployment
                </button>
              </div>
            }
          >
            <div class="bg-secondary-dark rounded-lg border border-secondary-medium overflow-hidden">
              <div class="px-6 py-4 border-b border-secondary-medium">
                <h2 class="text-lg font-medium text-white">Your Deployments</h2>
              </div>
              <div class="divide-y divide-secondary-medium">
                <For each={deployments()}>
                  {(deployment) => (
                    <div class="px-6 py-4 hover:bg-secondary-medium/50 transition-colors">
                      <div class="flex items-center justify-between">
                        <div class="flex-1">
                          <div class="flex items-center gap-3 mb-2">
                            <h3 class="text-white font-medium">{deployment.name}</h3>
                            <span
                              class={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium border ${getStatusStyle(deployment.status)}`}
                            >
                              {capitalizeStatus(deployment.status)}
                            </span>
                          </div>
                          <div class="flex items-center gap-4 text-sm text-gray-400">
                            <Show when={deployment.image}>
                              <span>Image: {deployment.image}</span>
                            </Show>
                            <span>Created: {formatDate(deployment.createdAt)}</span>
                            <Show when={deployment.lastDeployedAt}>
                              <span>Last deployed: {formatDate(deployment.lastDeployedAt!)}</span>
                            </Show>
                          </div>
                        </div>
                        <div class="flex items-center gap-2">
                          <Show when={deployment.url}>
                            <a
                              href={deployment.url}
                              target="_blank"
                              rel="noopener noreferrer"
                              class="text-primary hover:text-primary-light text-sm font-medium"
                            >
                              View App
                            </a>
                          </Show>
                          <button class="text-gray-400 hover:text-white p-1 rounded">
                            <svg class="w-5 h-5" fill="currentColor" viewBox="0 0 20 20">
                              <path d="M10 6a2 2 0 110-4 2 2 0 010 4zM10 12a2 2 0 110-4 2 2 0 010 4zM10 18a2 2 0 110-4 2 2 0 010 4z" />
                            </svg>
                          </button>
                        </div>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </div>
          </Show>
        </Show>
      </div>
  );
};

export default Deployments;