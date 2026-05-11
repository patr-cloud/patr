import { createFileRoute, useNavigate, useRouter } from "@tanstack/solid-router";
import { Title } from "@solidjs/meta";
import { createSignal, onMount, Show } from "solid-js";
import type { SocialLoginSetupRequest, SocialLoginSetupResponse } from "~/bindings";
import { Alert, Button, Input, InputType, useToast } from "~/components";
import { ButtonVariant } from "~/utils/color";
import { createAsyncAction, useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";

interface FieldErrors {
	email: string;
	firstName: string;
	lastName: string;
}

const emptyErrors: FieldErrors = {
	email: "",
	firstName: "",
	lastName: "",
};

const CompleteGithubSignup = () => {
	const [, setAuthState] = useAuthState();
	const router = useRouter();
	const navigate = useNavigate();
	const toast = useToast();

	const search = Route.useSearch();

	const [email, setEmail] = createSignal(search().email ?? "");
	const [firstName, setFirstName] = createSignal(search().firstName ?? "");
	const [lastName, setLastName] = createSignal(search().lastName ?? "");
	const [errors, setErrors] = createSignal<FieldErrors>({ ...emptyErrors });

	// Redirect to login if we arrive without a setup token
	onMount(() => {
		if (!search().setupToken) {
			toast("Invalid or expired GitHub sign-in session. Please try again.", "error");
			navigate({ to: "/login", replace: true });
		}
	});

	const clearError = (field: keyof FieldErrors) => {
		setErrors((prev) => ({ ...prev, [field]: "" }));
	};

	const validateInputs = (): boolean => {
		const newErrors = { ...emptyErrors };
		let valid = true;

		if (!email().trim()) {
			newErrors.email = "Email is required.";
			valid = false;
		}
		if (!firstName().trim()) {
			newErrors.firstName = "First name is required.";
			valid = false;
		}
		if (!lastName().trim()) {
			newErrors.lastName = "Last name is required.";
			valid = false;
		}
		setErrors(newErrors);
		return valid;
	};

	const { execute: submitSetup, isLoading } = createAsyncAction(async () => {
		if (!validateInputs()) return;

		const setupToken = search().setupToken;
		if (!setupToken) return;

		const body: SocialLoginSetupRequest = {
			setupToken,
			email: email(),
			firstName: firstName(),
			lastName: lastName(),
		};

		const resp = await httpRequest<SocialLoginSetupResponse>("/api/auth/social-login/github/setup", {
			method: "POST",
			body: JSON.stringify(body),
		});

		if (resp.ok) {
			const newAuth = {
				type: "LoggedIn" as const,
				accessToken: resp.data.accessToken,
				refreshToken: resp.data.refreshToken,
			};
			setAuthState(newAuth);
			router.update({
				...router.options,
				context: {
					...router.options.context,
					auth: newAuth,
				},
			});
			navigate({ to: "/", replace: true });
		} else {
			switch (resp.data?.error) {
				case "emailUnavailable":
					setErrors((prev) => ({ ...prev, email: "Email is already in use." }));
					break;
				case "socialLoginFailed":
					toast("Your GitHub session has expired. Please sign in with GitHub again.", "error");
					navigate({ to: "/login", replace: true });
					break;
				default:
					toast("Error creating account: " + resp.statusText, "error");
					break;
			}
		}
	});

	return (
		<>
			<Title>Complete Profile | Patr</Title>

			<form
				noValidate
				onSubmit={async (e) => {
					e.preventDefault();
					await submitSetup().catch(() => {
						toast("An unexpected error occurred. Please try again.", "error");
					});
				}}
				class="bg-secondary p-12 rounded-sm shadow-2xl w-full max-w-128 relative z-10 border border-secondary-medium"
			>
				{/* Header */}
				<div class="mb-6">
					<h1 class="font-bold text-2xl text-white mb-2">Complete your profile</h1>
					<p class="text-gray-400 text-sm">
						Your details have been pre-filled from GitHub — feel free to edit them.
					</p>
				</div>

				{/* Form */}
				<div>
					<Input
						type={InputType.Email}
						placeholder="Email"
						autocomplete="email"
						required={true}
						name="email"
						id="email"
						value={email}
						onInput={(e) => {
							setEmail(e.currentTarget.value);
							clearError("email");
						}}
						styleVariant="medium"
					/>
					<Show when={errors().email}>
						<div class="mt-1">
							<Alert message={errors().email} type="error" />
						</div>
					</Show>

					{/* Name Inputs */}
					<div class="flex items-center gap-4 mt-4">
						<div class="flex-1">
							<Input
								type={InputType.Text}
								placeholder="First Name"
								autocomplete="given-name"
								required={true}
								name="first-name"
								id="first-name"
								value={firstName}
								onInput={(e) => {
									setFirstName(e.currentTarget.value);
									clearError("firstName");
								}}
								styleVariant="medium"
							/>
							<Show when={errors().firstName}>
								<div class="mt-1">
									<Alert message={errors().firstName} type="error" />
								</div>
							</Show>
						</div>
						<div class="flex-1">
							<Input
								type={InputType.Text}
								placeholder="Last Name"
								autocomplete="family-name"
								required={true}
								name="last-name"
								id="last-name"
								value={lastName}
								onInput={(e) => {
									setLastName(e.currentTarget.value);
									clearError("lastName");
								}}
								styleVariant="medium"
							/>
							<Show when={errors().lastName}>
								<div class="mt-1">
									<Alert message={errors().lastName} type="error" />
								</div>
							</Show>
						</div>
					</div>

					<p class="mt-3 text-xs text-gray-500 leading-relaxed">
						Already have a Patr account? Log in first, then connect GitHub from your profile's Connected
						Accounts section.
					</p>

					{/* Submit */}
					<div class="pt-8 w-full flex justify-end">
						<Button
							variant={ButtonVariant.Contained}
							class="py-4 text-base font-semibold px-8"
							type="submit"
							loading={isLoading}
							loadingContent={() => <span>Creating account...</span>}
						>
							Create Account
						</Button>
					</div>
				</div>
			</form>

			{/* Footer */}
			<div class="absolute bottom-6 left-0 right-0 text-center">
				<p class="text-gray-500 text-xs">&copy; {new Date().getFullYear()} Patr. All rights reserved.</p>
			</div>
		</>
	);
};

export const Route = createFileRoute("/_logged-out/sign-up/github")({
	validateSearch: (
		search: Record<string, unknown>
	): {
		setupToken?: string;
		firstName?: string;
		lastName?: string;
		email?: string;
	} => ({
		setupToken: (search.setupToken as string) || undefined,
		firstName: (search.firstName as string) || undefined,
		lastName: (search.lastName as string) || undefined,
		email: (search.email as string) || undefined,
	}),
	component: CompleteGithubSignup,
});
