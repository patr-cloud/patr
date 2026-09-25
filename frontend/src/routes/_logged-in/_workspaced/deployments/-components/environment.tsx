import { useQueryClient } from "@tanstack/solid-query";
import { createEffect, createSignal, Show } from "solid-js";
import { GetDeploymentInfoResponse, UpdateDeploymentResponse } from "~/bindings";
import { Button, useToast } from "~/components";
import { useAuthState } from "~/hooks";
import { useDeploymentInfoQuery } from "~/hooks/fetch";
import { useGetPermissions } from "~/hooks/is-allowed";
import { deploymentKeys } from "~/hooks/query-keys";
import { useLastWorkspaceId } from "~/hooks/state-hooks";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import EnvList from "./env-list";
import { toUpdateRequest } from "./utils";

interface DeploymentEnvironmentProps {
	deploymentId: string;
}

/** The deployment's environment configuration, on its own tab. */
const DeploymentEnvironment = (props: DeploymentEnvironmentProps) => {
	const [authState] = useAuthState();
	const [workspaceId] = useLastWorkspaceId();
	const toast = useToast();
	const queryClient = useQueryClient();

	const deploymentQuery = useDeploymentInfoQuery(() => props.deploymentId);
	const deploymentPermissions = useGetPermissions("deployment", () => props.deploymentId);

	// Local draft, seeded once from the query. The update sends the whole
	// deployment, so the fields this tab doesn't touch ride along unchanged.
	const [localInfo, setLocalInfo] = createSignal<GetDeploymentInfoResponse | undefined>(undefined);

	createEffect(() => {
		if (deploymentQuery.data && !localInfo()) {
			setLocalInfo(deploymentQuery.data);
		}
	});

	const [isUpdating, setIsUpdating] = createSignal(false);
	const [envValid, setEnvValid] = createSignal(true);

	const refetchDeploymentInfo = () => {
		const wsId = workspaceId();
		if (wsId) {
			queryClient.invalidateQueries({ queryKey: deploymentKeys.detail(wsId, props.deploymentId) });
		}
	};

	const onSubmitUpdate = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
		e.preventDefault();
		const auth = authState();
		if (!auth || auth.type !== "LoggedIn") {
			toast("User not logged in", "error");
			return;
		}

		if (!envValid()) {
			toast("Please fix the highlighted errors before saving", "error");
			return;
		}

		const info = localInfo();
		if (!info) {
			toast("Deployment info not available", "error");
			return;
		}

		setIsUpdating(true);
		try {
			const response = await httpRequest<UpdateDeploymentResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId()}/deployment/${info.id}`,
				{
					method: "PATCH",
					body: JSON.stringify(toUpdateRequest(info)),
				}
			);

			if (!response.ok) {
				console.error("Failed to update deployment:", response.data.error);
				toast("Failed to update deployment", "error");
				refetchDeploymentInfo();
				return;
			}

			toast("Deployment updated successfully", "success");
			refetchDeploymentInfo();
		} finally {
			setIsUpdating(false);
		}
	};

	return (
		<form onSubmit={onSubmitUpdate} class="flex flex-col gap-6 justify-between w-full flex-1">
			<div class="flex flex-col gap-4 items-start w-full">
				<div class="flex flex-col gap-1 w-full">
					<h2 class="text-lg text-white font-semibold">Environment Variables</h2>
					<p class="text-sm text-grey">
						Passed to the container on startup. Anything sensitive can be stored as a workspace secret
						instead, so the value never lives on the deployment.
					</p>
				</div>

				<EnvList
					disabled={() => !deploymentPermissions().edit}
					value={() => deploymentQuery.data?.environmentVariables ?? {}}
					onChange={(next) =>
						setLocalInfo((prev) => (prev ? { ...prev, environmentVariables: next } : undefined))
					}
					onValidityChange={setEnvValid}
				/>
			</div>

			<Show when={deploymentPermissions().edit}>
				<div class="w-full flex justify-end items-center">
					<Button
						disabled={!deploymentPermissions().edit || isUpdating() || !envValid()}
						loading={isUpdating()}
						loadingContent={() => <span>Updating...</span>}
						type="submit"
						variant="contained"
					>
						Update
					</Button>
				</div>
			</Show>
		</form>
	);
};

export default DeploymentEnvironment;
