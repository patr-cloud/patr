import { createSignal } from "solid-js";
import { useQueryClient } from "@tanstack/solid-query";
import { CreateWorkspaceResponse } from "~/bindings";
import { useToast } from "~/components/toast";
import { createAsyncAction } from "./actions";
import { useAuthState, useLastWorkspaceId } from "./state-hooks";
import { workspacesKeys } from "./query-keys";
import { httpRequest } from "~/utils/http-request";

/** Options for {@link useCreateWorkspace}. */
interface UseCreateWorkspaceOptions {
	/**
	 * Called after the workspace is created, selected, and the workspaces list
	 * has been invalidated. The first-workspace screen leaves this empty (the
	 * layout reactively swaps the onboarding screen for the dashboard once the
	 * list refetches); the settings page uses it to navigate.
	 */
	onCreated?: (id: string, name: string) => void;
}

/**
 * Shared create-workspace flow used by both the first-workspace onboarding
 * screen and the "new workspace" settings page. Owns the name/error/loading
 * state and the `POST → select → invalidate` sequence so the two call sites
 * differ only in their surrounding layout and what they do afterwards.
 */
export function useCreateWorkspace(options: UseCreateWorkspaceOptions = {}) {
	const [authState] = useAuthState();
	const [, setWorkspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const queryClient = useQueryClient();

	const [workspaceName, setWorkspaceName] = createSignal("");
	const [nameError, setNameError] = createSignal("");

	const { execute: submit, isLoading } = createAsyncAction(async () => {
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to create a workspace", "error");
			return;
		}

		const name = workspaceName().trim();
		if (!name) {
			setNameError("Workspace name is required.");
			return;
		}

		const response = await httpRequest<CreateWorkspaceResponse>(`${import.meta.env.VITE_BASE_URL}/api/workspace`, {
			method: "POST",
			body: JSON.stringify({ name }),
		});

		if (!response.ok) {
			setNameError("Failed to create workspace. Please try a different name.");
			return;
		}

		toast("Workspace created successfully", "success");

		if (response.data.id) {
			setWorkspaceId(response.data.id);
			await queryClient.invalidateQueries({ queryKey: workspacesKeys.list() });
			options.onCreated?.(response.data.id, name);
		}
	});

	return { workspaceName, setWorkspaceName, nameError, setNameError, isLoading, submit };
}
