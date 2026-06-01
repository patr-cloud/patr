import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal } from "solid-js";
import { AddRunnerToWorkspaceResponse } from "~/bindings";
import {
	Button,
	ButtonVariant,
	Input,
	InputType,
	InputLabel,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	useToast,
} from "~/components";
import { createFormAction } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import { validateRunnerName } from "~/utils/validation";

const CreateRunnerPage = () => {
	const [name, setName] = createSignal<string>("");
	const [nameError, setNameError] = createSignal("");
	const navigate = useNavigate();
	const toast = useToast();

	const checkName = (): boolean => {
		const r = validateRunnerName(name());
		setNameError(r.valid ? "" : (r.error ?? ""));
		return r.valid;
	};

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId }) => {
		if (!checkName()) return;

		const response = await httpRequest<AddRunnerToWorkspaceResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${workspaceId}/runner`,
			{
				method: "POST",
				body: JSON.stringify({
					name: name(),
				}),
			}
		);

		if (!response.ok) {
			setNameError("Failed to create runner. Please try a different name.");
			return;
		}

		toast("Runner created successfully", "success");
		navigate({ to: "/runners" });
	});

	return (
		<>
			<Title>New Runner | Patr</Title>
			<PageContainer>
				<PageContainerHead
					subText="Runners execute deployments on your machines or clusters"
					breadcrumbs={[
						{
							label: "Runners",
							url: "/runners",
						},
						{
							label: "Add",
						},
					]}
				/>
				<PageContainerBody class="flex flex-col">
					<form noValidate onSubmit={onSubmit} class="flex flex-col gap-8 w-full">
						<div class="flex gap-8 items-center w-full">
							<InputLabel parentClass="flex-2" for="runner-name" label="Runner Name" />
							<div class="flex-10 flex flex-col">
								<Input
									id="runner-name"
									name="runner-name"
									placeholder="Enter Runner Name"
									type={InputType.Text}
									value={name()}
									onInput={(e) => {
										setName(e.currentTarget.value);
										setNameError("");
									}}
									onBlur={() => checkName()}
									error={nameError}
								/>
							</div>
						</div>

						<div class="w-full flex justify-end">
							<Button
								loading={isLoading}
								loadingContent={() => <span>Creating Runner...</span>}
								variant={ButtonVariant.Contained}
								type="submit"
							>
								Create Runner
							</Button>
						</div>
					</form>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/runners/new")({
	component: CreateRunnerPage,
});
