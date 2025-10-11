import { createSignal, Show, onMount } from "solid-js";
import { useNavigate } from "@solidjs/router";
import Button from "~/components/button";
import Input, { InputVariants } from "~/components/input";
import ErrorMessage from "~/components/error-message";
import LoadingSpinner from "~/components/loading-spinner";
import { ButtonVariant } from "~/utils/color";
import { api, type WorkspaceData } from "~/utils/api";
import { ValidationUtil } from "~/utils/validation";
import { LocalStorage } from "~/utils/storage";
import { useAuthState } from "~/utils/state";

const CreateWorkspace = () => {
  const navigate = useNavigate();
  const [authState] = useAuthState();

  // Form state
  const [name, setName] = createSignal("");
  const [description, setDescription] = createSignal("");
  const [nameError, setNameError] = createSignal<string | undefined>();
  const [descriptionError, setDescriptionError] = createSignal<string | undefined>();
  const [isSubmitting, setIsSubmitting] = createSignal(false);
  const [apiError, setApiError] = createSignal<string | undefined>();

  // Check authentication on mount
  onMount(() => {
    if (authState().type !== "LoggedIn") {
      navigate("/login", { replace: true });
    }
  });

  // Validation functions
  const validateName = (value: string) => {
    const result = ValidationUtil.validateWorkspaceName(value);
    setNameError(result.isValid ? undefined : result.error);
    return result.isValid;
  };

  const validateDescription = (value: string) => {
    const result = ValidationUtil.validateWorkspaceDescription(value);
    setDescriptionError(result.isValid ? undefined : result.error);
    return result.isValid;
  };

  // Form handlers
  const handleNameInput = (e: Event) => {
    const target = e.target as HTMLInputElement;
    const value = target.value;
    setName(value);
    
    // Clear API error when user starts typing
    if (apiError()) {
      setApiError(undefined);
    }
    
    // Validate on input for real-time feedback
    validateName(value);
  };

  const handleDescriptionInput = (e: Event) => {
    const target = e.target as HTMLInputElement;
    const value = target.value;
    setDescription(value);
    
    // Clear API error when user starts typing
    if (apiError()) {
      setApiError(undefined);
    }
    
    // Validate on input for real-time feedback
    validateDescription(value);
  };

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    
    // Clear previous errors
    setApiError(undefined);
    
    // Validate all fields
    const isNameValid = validateName(name());
    const isDescriptionValid = validateDescription(description());
    
    if (!isNameValid || !isDescriptionValid) {
      return;
    }

    setIsSubmitting(true);

    try {
      const workspaceData: WorkspaceData = {
        name: name().trim(),
        description: description().trim() || undefined,
      };

      const result = await api.createWorkspace(workspaceData);

      if (result.success) {
        // Store workspace in local storage
        LocalStorage.setCurrentWorkspace(result.id);
        
        // Navigate to deployments page
        navigate("/deployments", { replace: true });
      } else {
        setApiError(result.message || "Failed to create workspace");
      }
    } catch (error) {
      setApiError("An unexpected error occurred. Please try again.");
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleRetry = () => {
    setApiError(undefined);
  };

  return (
    <div class="max-w-2xl mx-auto">
        {/* Header */}
        <div class="mb-8">
          <h1 class="text-3xl font-bold text-white mb-2">Create Your Workspace</h1>
          <p class="text-gray-400">
            Set up your workspace to start managing deployments and applications.
          </p>
        </div>

        {/* Form */}
        <div class="bg-secondary-dark rounded-lg border border-secondary-medium p-6">
          <form onSubmit={handleSubmit} class="space-y-6">
            {/* API Error */}
            <Show when={apiError()}>
              <ErrorMessage
                message={apiError()!}
                dismissible={true}
                showRetry={true}
                onRetry={handleRetry}
                onDismiss={() => setApiError(undefined)}
              />
            </Show>

            {/* Workspace Name */}
            <div class="space-y-2">
              <label for="workspace-name" class="block text-sm font-medium text-white">
                Workspace Name *
              </label>
              <Input
                id="workspace-name"
                name="name"
                type={InputVariants.Text}
                placeholder="Enter workspace name"
                value={name}
                onInput={handleNameInput}
                disabled={isSubmitting()}
                required
              />
              <Show when={nameError()}>
                <p class="text-sm text-error">{nameError()}</p>
              </Show>
              <p class="text-xs text-gray-500">
                Choose a descriptive name for your workspace (2-50 characters)
              </p>
            </div>

            {/* Workspace Description */}
            <div class="space-y-2">
              <label for="workspace-description" class="block text-sm font-medium text-white">
                Description (Optional)
              </label>
              <div class="rounded-xs flex justify-start items-start border border-secondary-medium px-sm transition-all duration-250 focus-within:border-primary focus-within:shadow-md bg-secondary-light">
                <textarea
                  id="workspace-description"
                  name="description"
                  placeholder="Describe your workspace (optional)"
                  value={description()}
                  onInput={handleDescriptionInput}
                  disabled={isSubmitting()}
                  rows="3"
                  class="overflow-hidden text-sm w-full text-white font-thin border-none bg-transparent disabled:text-disabled focus:outline-none placeholder:text-grey py-xs px-lg resize-none"
                />
              </div>
              <Show when={descriptionError()}>
                <p class="text-sm text-error">{descriptionError()}</p>
              </Show>
              <p class="text-xs text-gray-500">
                Provide additional context about this workspace (max 500 characters)
              </p>
            </div>

            {/* Submit Button */}
            <div class="flex justify-end space-x-4 pt-4">
              <Button
                type="button"
                variant={ButtonVariant.Plain}
                class="text-gray-400 hover:text-white"
                disabled={isSubmitting()}
                onClick={() => navigate("/login")}
              >
                Back to Login
              </Button>
              
              <Button
                type="submit"
                variant={ButtonVariant.Contained}
                disabled={isSubmitting() || !!nameError() || !!descriptionError()}
                class="min-w-[120px] flex items-center justify-center"
              >
                <Show
                  when={!isSubmitting()}
                  fallback={
                    <div class="flex items-center space-x-2">
                      <LoadingSpinner size="sm" />
                      <span>Creating...</span>
                    </div>
                  }
                >
                  Create Workspace
                </Show>
              </Button>
            </div>
          </form>
        </div>

        {/* Additional Info */}
        <div class="mt-8 p-4 bg-secondary-light rounded-lg border border-secondary-medium">
          <h3 class="text-sm font-medium text-white mb-2">What's Next?</h3>
          <ul class="text-sm text-gray-400 space-y-1">
            <li>• After creating your workspace, you'll be able to manage deployments</li>
            <li>• You can create multiple workspaces to organize different projects</li>
            <li>• Invite team members to collaborate on your workspace</li>
          </ul>
        </div>
      </div>
  );
};

export default CreateWorkspace;