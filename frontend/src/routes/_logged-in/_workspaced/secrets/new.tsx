import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, Show } from "solid-js";
import {
	Alert,
	PageContainer,
	PageContainerBody,
	PageContainerHead,
	ButtonVariant,
	Button,
	Input,
	InputType,
	PasswordInput,
	useToast,
	Label,
} from "~/components";
import { createFormAction } from "~/hooks";
import { CreateSecretRequest, CreateSecretResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import { cloudOnly } from "~/utils/env";

const CreateSecretPage = () => {
	const [name, setName] = createSignal("");
	const [value, setValue] = createSignal("");
	const [error, setError] = createSignal("");
	const navigate = useNavigate();
	const toast = useToast();

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId: wsId }) => {
		const secretName = name().trim();

		if (!secretName) {
			setError("Name is required.");
			return;
		}

		if (!value()) {
			setError("Value is required.");
			return;
		}

		const requestBody: CreateSecretRequest = {
			name: secretName,
			value: value(),
		};

		const response = await httpRequest<CreateSecretResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/secret`,
			{
				method: "POST",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			setError("Failed to create secret. Please try again.");
			return;
		}

		toast("Secret created successfully", "success");
		navigate({ to: "/secrets" });
	});

	return (
		<>
			<Title>Add Secret | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Secrets",
							url: "/secrets",
						},
						{
							label: "Add",
						},
					]}
					subText="Store sensitive values securely and reference them from your deployments."
				/>
				<PageContainerBody class="flex flex-col">
					<form noValidate onSubmit={onSubmit} class="flex flex-col gap-8 w-full">
						<div class="flex flex-col gap-4 w-full">
							<div class="flex gap-8 items-start w-full">
								<Label parentClass="flex-2 pt-2.5" for="secret-name" label="Name" />
								<div class="flex-10 flex flex-col">
									<Input
										id="secret-name"
										name="secret-name"
										placeholder="OPENAI_API_KEY"
										type={InputType.Text}
										value={name()}
										onInput={(e) => {
											setName(e.currentTarget.value);
											setError("");
										}}
									/>
								</div>
							</div>
							<div class="flex gap-8 items-start w-full">
								<Label parentClass="flex-2 pt-2.5" for="secret-value" label="Value" />
								<div class="flex-10 flex flex-col">
									<PasswordInput
										id="secret-value"
										name="secret-value"
										placeholder="Enter the secret value"
										value={value()}
										onInput={(e) => {
											setValue(e.currentTarget.value);
											setError("");
										}}
									/>
									<Show when={error()}>
										<div class="mt-1">
											<Alert message={error()} type="error" />
										</div>
									</Show>
								</div>
							</div>
						</div>

						<div class="w-full flex justify-end">
							<Button
								variant={ButtonVariant.Contained}
								type="submit"
								loading={isLoading}
								loadingContent={() => <span>Creating...</span>}
							>
								Add Secret
							</Button>
						</div>
					</form>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/secrets/new")(
	cloudOnly({
		component: CreateSecretPage,
	})
);
