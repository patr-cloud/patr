import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal } from "solid-js";
import { AddRunnerToWorkspaceResponse } from "~/bindings";
import { Button, ButtonVariant, PageContainer, PageContainerBody, PageContainerHead, useToast } from "~/components";
import Input, { InputType } from "~/components/input";
import InputLabel from "~/components/input-label";
import { createFormAction } from "~/hooks";
import { httpRequest } from "~/utils/http-request";

const CreateRunnerPage = () => {
	const [name, setName] = createSignal<string>("");
	const navigate = useNavigate();
	const toast = useToast();

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId }) => {
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
			console.error("Failed to create runner:", response.data.error);
			toast("Failed to create runner", "error");
			return;
		}

		toast("Runner created successfully", "success");
		setName("");
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
							label: "Create",
						},
					]}
				/>
				<PageContainerBody class="flex flex-col justify-between gap-8">
					<form onSubmit={onSubmit} class="flex flex-col gap-8 items-start w-full justify-between flex-1">
						<div class="flex w-full flex-col justify-between gap-6 h-full flex-1">
							<div class="flex flex-col gap-6 items-start w-full">
								<div class="flex gap-8 items-center w-full">
									<InputLabel parentClass="flex-2" for="runner-name" label="Runner Name" />
									<Input
										class="flex-10"
										name="runner-name"
										placeholder="Enter Runner Name"
										type={InputType.Text}
										value={name()}
										onInput={(e) => {
											setName(e.currentTarget.value);
										}}
									/>
								</div>
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
