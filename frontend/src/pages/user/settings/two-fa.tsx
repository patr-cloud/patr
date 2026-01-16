import { createEffect, createResource, createSignal, onCleanup, Suspense } from "solid-js";
import { Button, ButtonVariant, useToast } from "~/components";
import { ModalContainer } from "~/components/modal";
import OtpInput from "~/components/otp-input";
import { useAuthState } from "~/hooks";
import { GetMfaSecretResponse, GetUserInfoResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";
import { VsRefresh } from "solid-icons/vs";

interface ModalContainerProps {
	isMfaEnabled: boolean;
	closeFn: (prev: boolean) => void;
	refetchUserInfo: () => GetUserInfoResponse | Promise<GetUserInfoResponse | undefined> | null | undefined;
}

const TwoFactorAuthModal = (props: ModalContainerProps) => {
	const [authState] = useAuthState();
	const toast = useToast();

	const [otp, setOtp] = createSignal("");
	const [timeRemaining, setTimeRemaining] = createSignal(5 * 60); // 15 minutes in seconds
	const [isExpired, setIsExpired] = createSignal(false);

	const fetchMfaSecret = async (auth: ReturnType<typeof authState>) => {
		if (props.isMfaEnabled) return undefined;
		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to enable 2FA", "error");
			return undefined;
		}

		const response = await httpRequest<GetMfaSecretResponse>(`${import.meta.env.VITE_BASE_URL}/api/user/mfa`, {
			method: "GET",
			headers: {
				"Content-Type": "application/json",
				Authorization: `Bearer ${auth.accessToken}`,
			},
		});

		if (!response.ok) {
			toast(`Failed to fetch MFA secret: ${response.data.error || "Unknown error"}`, "error");
			return undefined;
		}

		return response.data;
	};

	const [mfaSecret, { refetch }] = createResource(authState(), fetchMfaSecret);

	// Timer countdown
	createEffect(() => {
		if (isExpired()) return;

		const interval = setInterval(() => {
			setTimeRemaining((prev) => {
				if (prev <= 1) {
					setIsExpired(true);
					clearInterval(interval);
					return 0;
				}
				return prev - 1;
			});
		}, 1000);

		onCleanup(() => clearInterval(interval));
	});

	const formatTime = (seconds: number) => {
		const mins = Math.floor(seconds / 60);
		const secs = seconds % 60;
		return `${mins}:${secs.toString().padStart(2, "0")}`;
	};

	const handleReload = () => {
		setTimeRemaining(15 * 60);
		setIsExpired(false);
		refetch();
	};

	const onVerifyOtp = async (e: Event) => {
		e.preventDefault();
		const auth = authState();

		if (!auth || auth.type !== "LoggedIn") {
			toast("You must be logged in to verify 2FA", "error");
			return;
		}

		const response = await httpRequest(`${import.meta.env.VITE_BASE_URL}/api/user/mfa`, {
			method: props.isMfaEnabled ? "DELETE" : "POST",
			headers: {
				"Content-Type": "application/json",
				Authorization: `Bearer ${auth.accessToken}`,
			},
			body: JSON.stringify({ otp: otp() }),
		});

		if (!response.ok) {
			toast(`Failed to verify OTP: ${response.data.error || "Unknown error"}`, "error");
			props.closeFn(false);
			return;
		}

		toast("Two-Factor Authentication enabled successfully!", "success");
		props.refetchUserInfo();
		props.closeFn(false);
	};

	return (
		<ModalContainer width="600px" closeFn={props.closeFn} class="flex flex-col gap-3 items-center">
			{
				/* Modal Title */
				props.isMfaEnabled ? (
					<h2 class="text-2xl font-semibold mb-4 text-primary">To Disable 2FA, Verify OTP</h2>
				) : (
					<h2 class="text-2xl font-semibold mb-4 text-primary">Secure Your Account</h2>
				)
			}
			{!props.isMfaEnabled && (
				<>
					<p class="text-center text-white">
						Scan the QR code below with your preferred authenticator app and then enter the provided one time code below
					</p>

					{/* Timer Display */}
					<div class="text-white text-sm font-medium">
						{isExpired() ? (
							<span class="text-error">QR Code Expired</span>
						) : (
							<span>Time remaining: {formatTime(timeRemaining())}</span>
						)}
					</div>

					<div class="border border-border-color p-4 rounded-xs min-h-[264px] min-w-[264px] flex justify-center items-center relative">
						<Suspense fallback={<div class="text-white">Loading QR Code...</div>}>
							<img
								style={{
									height: "230px",
									filter: isExpired() ? "blur(8px)" : "none",
									transition: "filter 0.3s ease",
								}}
								width="auto"
								src={`data:image/png;base64,${mfaSecret.latest?.qr || ""}`}
							/>
							{isExpired() && (
								<div class="absolute inset-0 flex items-center justify-center">
									<Button variant={ButtonVariant.Contained} onClick={handleReload} class="z-10 hover:text-primary">
										<VsRefresh class="text-secondary hover:text-primary" size={48} />
									</Button>
								</div>
							)}
						</Suspense>
					</div>

					<div class="w-full flex items-center gap-2">
						<div class="h-px w-full bg-grey/20"></div>
						<span class="text-white">THEN</span>
						<div class="h-px w-full bg-grey/20"></div>
					</div>
				</>
			)}
			<form onSubmit={onVerifyOtp}>
				<OtpInput
					inputVariant="medium"
					otpDigits={() => otp().split("")}
					setOtpDigits={(digits) => setOtp(digits.join(""))}
				/>

				<Button
					variant={ButtonVariant.Contained}
					type="submit"
					class="mt-4 w-full"
					disabled={otp().length < 6 || isExpired()}
				>
					VERIFY
				</Button>
			</form>
		</ModalContainer>
	);
};

export default TwoFactorAuthModal;
