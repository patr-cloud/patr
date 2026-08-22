import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { useQueryClient } from "@tanstack/solid-query";
import { createEffect, createSignal, ErrorBoundary, Show, Suspense } from "solid-js";
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
	LoadingSpinner,
} from "~/components";
import { createFormAction } from "~/hooks";
import { useSecretInfoQuery } from "~/hooks/fetch";
import { secretKeys } from "~/hooks/query-keys";
import { UpdateSecretRequest, UpdateSecretResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import { cloudOnly } from "~/utils/env";
import { formatRelativeTime } from "~/utils/func";

const SecretDetailPage = () => {
	const params = Route.useParams();
	const navigate = useNavigate();
	const toast = useToast();
	const queryClient = useQueryClient();

	const secretInfoQuery = useSecretInfoQuery(() => params().id);

	const [name, setName] = createSignal("");
	const [value, setValue] = createSignal("");
	const [error, setError] = createSignal("");

	// Seed the editable name once the secret info loads.
	createEffect(() => {
		const secretName = secretInfoQuery.data?.secret.name;
		if (secretName !== undefined) {
			setName(secretName);
		}
	});

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId: wsId }) => {
		const secretName = name().trim();

		if (!secretName) {
			setError("Name is required.");
			return;
		}

		const requestBody: UpdateSecretRequest = {
			name: secretName,
			value: value() ? value() : null,
		};

		const response = await httpRequest<UpdateSecretResponse>(
			`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/secret/${params().id}`,
			{
				method: "PATCH",
				body: JSON.stringify(requestBody),
			}
		);

		if (!response.ok) {
			setError("Failed to update secret. Please try again.");
			return;
		}

		queryClient.invalidateQueries({ queryKey: secretKeys.detail(wsId, params().id) });
		queryClient.invalidateQueries({ queryKey: secretKeys.all(wsId) });
		setValue("");
		toast("Secret updated successfully", "success");
		navigate({ to: "/secrets" });
	});

	return (
		<>
			<Title>Secret | Patr</Title>
			<PageContainer>
				<PageContainerHead
					breadcrumbs={[
						{
							label: "Secrets",
							url: "/secrets",
						},
						{
							label: secretInfoQuery.data?.secret.name ?? "Details",
						},
					]}
					subText="View and update this secret."
				/>
				<PageContainerBody class="flex flex-col">
					<ErrorBoundary
						fallback={(err, reset) => (
							<div class="flex flex-col items-center justify-center gap-4 py-16">
								<p class="text-error text-sm">Error loading secret: {err.message}</p>
								<Button variant={ButtonVariant.Outlined} onClick={reset}>
									Retry
								</Button>
							</div>
						)}
					>
						<Suspense
							fallback={
								<div class="flex items-center justify-center gap-2 py-16 text-grey">
									<LoadingSpinner size={20} />
									<span class="text-sm">Loading secret...</span>
								</div>
							}
						>
							<Show when={secretInfoQuery.data?.secret}>
								{(secret) => (
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
												<Label parentClass="flex-2 pt-2.5" label="Created" />
												<p class="flex-10 text-white text-sm pt-2.5">
													{formatRelativeTime(secret().created)}
												</p>
											</div>

											<div class="flex gap-8 items-start w-full">
												<Label parentClass="flex-2 pt-2.5" label="Last updated" />
												<p class="flex-10 text-white text-sm pt-2.5">
													{formatRelativeTime(secret().lastUpdated)}
												</p>
											</div>

											<div class="flex gap-8 items-start w-full">
												<Label parentClass="flex-2 pt-2.5" for="secret-value" label="Value" />
												<div class="flex-10 flex flex-col">
													<PasswordInput
														id="secret-value"
														name="secret-value"
														placeholder="Enter a new value to rotate the secret"
														value={value()}
														onInput={(e) => {
															setValue(e.currentTarget.value);
															setError("");
														}}
													/>
													<p class="text-grey text-xs mt-1">
														Leave blank to keep the current value.
													</p>
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
												loadingContent={() => <span>Saving...</span>}
											>
												Save Changes
											</Button>
										</div>
									</form>
								)}
							</Show>
						</Suspense>
					</ErrorBoundary>
				</PageContainerBody>
			</PageContainer>
		</>
	);
};

export const Route = createFileRoute("/_logged-in/_workspaced/secrets/$id")(
	cloudOnly({
		component: SecretDetailPage,
	})
);
