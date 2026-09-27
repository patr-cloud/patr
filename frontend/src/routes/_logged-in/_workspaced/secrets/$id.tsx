import { createFileRoute, useNavigate } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { useQueryClient } from "@tanstack/solid-query";
import { createEffect, createSignal, ErrorBoundary, Show, Suspense } from "solid-js";
import {
	Alert,
	DeleteModal,
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
import { createAuthenticatedAction, createFormAction, useIsAllowed } from "~/hooks";
import { useSecretInfoQuery } from "~/hooks/fetch";
import { secretKeys } from "~/hooks/query-keys";
import { DeleteSecretResponse, UpdateSecretRequest, UpdateSecretResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import { formatRelativeTime } from "~/utils/func";

const SecretDetailPage = () => {
	const params = Route.useParams();
	const navigate = useNavigate();
	const toast = useToast();
	const queryClient = useQueryClient();

	const secretInfoQuery = useSecretInfoQuery(() => params().id);
	const isDeleteAllowed = useIsAllowed("secret", "delete", () => params().id);

	const [name, setName] = createSignal<string>();
	const [value, setValue] = createSignal("");
	// The value can't be read back, so its field stays hidden until the user
	// asks to replace it.
	const [editingValue, setEditingValue] = createSignal(false);
	const [error, setError] = createSignal("");

	// Seed the editable name once the secret info loads. Only once: the query
	// refetches whenever the window regains focus, and reseeding then would wipe
	// out a rename the user is in the middle of typing.
	createEffect(() => {
		const secretName = secretInfoQuery.data?.secret.name;
		if (secretName !== undefined && name() === undefined) {
			setName(secretName);
		}
	});

	const { onSubmit, isLoading } = createFormAction(async ({ workspaceId: wsId }) => {
		const secretName = (name() ?? "").trim();

		if (!secretName) {
			setError("Name is required.");
			return;
		}

		const requestBody: UpdateSecretRequest = {
			name: secretName,
			value: editingValue() && value() ? value() : undefined,
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

	const { execute: onClickDelete, isLoading: deleteLoading } = createAuthenticatedAction(
		async ({ workspaceId: wsId }) => {
			const response = await httpRequest<DeleteSecretResponse>(
				`${import.meta.env.VITE_BASE_URL}/api/workspace/${wsId}/secret/${params().id}`,
				{ method: "DELETE" }
			);

			if (!response.ok) {
				console.error("Failed to delete secret:", response.data.error);
				if (response.data.error === "resourceInUse") {
					toast("Cannot delete secret: Secret is in use by deployment(s)", "error");
					return;
				}
				toast("Failed to delete secret", "error");
				return;
			}

			queryClient.invalidateQueries({ queryKey: secretKeys.all(wsId) });
			toast("Secret deleted successfully", "success");
			navigate({ to: "/secrets" });
		}
	);

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
					actions={() => (
						<Show when={isDeleteAllowed() && secretInfoQuery.data?.secret.name}>
							<DeleteModal
								isLoading={deleteLoading()}
								title="Delete Secret"
								onClickDelete={(e) => {
									e.preventDefault();
									onClickDelete();
								}}
								resourceName={secretInfoQuery.data?.secret.name ?? ""}
							/>
						</Show>
					)}
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
														value={name() ?? ""}
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
													<Show
														when={editingValue()}
														fallback={
															<div class="flex items-center gap-4 pt-2.5">
																<p class="text-grey text-sm">
																	The current value is hidden and can't be viewed. It
																	stays unchanged unless you update it.
																</p>
																<Button
																	type="button"
																	variant={ButtonVariant.Plain}
																	onClick={() => setEditingValue(true)}
																	class="text-sm whitespace-nowrap cursor-pointer"
																>
																	Update value
																</Button>
															</div>
														}
													>
														<PasswordInput
															id="secret-value"
															name="secret-value"
															autocomplete="new-password"
															placeholder="Enter the new value"
															value={value()}
															onInput={(e) => {
																setValue(e.currentTarget.value);
																setError("");
															}}
														/>
														<div class="flex items-center gap-4 mt-1">
															<p class="text-grey text-xs">
																Saving replaces the current value. Deployments using
																this secret restart with the new one.
															</p>
															<Button
																type="button"
																variant={ButtonVariant.Plain}
																onClick={() => {
																	setEditingValue(false);
																	setValue("");
																}}
																class="text-xs whitespace-nowrap cursor-pointer"
															>
																Cancel
															</Button>
														</div>
													</Show>
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

export const Route = createFileRoute("/_logged-in/_workspaced/secrets/$id")({
	component: SecretDetailPage,
});
