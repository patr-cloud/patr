import { createEffect, createResource, createSignal, Suspense } from "solid-js";
import { Button, ButtonVariant, useToast } from "~/components";
import { ModalContainer } from "~/components/modal";
import QRCode from "qrcode";
import { TOTP } from "totp-generator";
import OtpInput from "~/components/otp-input";
import { useAuthState } from "~/hooks";
import { GetMfaSecretResponse, GetUserInfoResponse } from "~/bindings";
import { httpRequest } from "~/utils/http-request";

interface ModalContainerProps {
  closeFn: (prev: boolean) => void;
  refetchUserInfo: () =>
    | GetUserInfoResponse
    | Promise<GetUserInfoResponse | undefined>
    | null
    | undefined;
}

const TwoFactorAuthModal = (props: ModalContainerProps) => {
  const [authState] = useAuthState();
  const toast = useToast();

  const [otp, setOtp] = createSignal("");
  const [qrDataUrl, setQrDataUrl] = createSignal<string | null>(null);
  // Function to generate the QR code
  const generateQrCode = async (text: string) => {
    try {
      const url = await QRCode.toDataURL(text, {
        width: 230,
        margin: 1,
      });
      console.log("Generated QR Code URL:", url);
      setQrDataUrl(url);
    } catch (err) {
      console.error(err);
    }
  };

  const [mfaSecret] = createResource(authState(), async (auth) => {
    if (!auth || auth.type !== "LoggedIn") {
      toast("You must be logged in to enable 2FA", "error");
      return undefined;
    }

    const response = await httpRequest<GetMfaSecretResponse>(
      `${import.meta.env.VITE_BASE_URL}/api/user/mfa`,
      {
        method: "GET",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
      }
    );

    if (!response.ok) {
      toast(
        `Failed to fetch MFA secret: ${response.data.error || "Unknown error"}`,
        "error"
      );
      return undefined;
    }

    return response.data;
  });

  const onVerifyOtp = async (e: Event) => {
    e.preventDefault();
    const auth = authState();

    if (!auth || auth.type !== "LoggedIn") {
      toast("You must be logged in to verify 2FA", "error");
      return;
    }

    const response = await httpRequest(
      `${import.meta.env.VITE_BASE_URL}/api/user/mfa`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Authorization: `Bearer ${auth.accessToken}`,
        },
        body: JSON.stringify({ otp: otp() }),
      }
    );

    if (!response.ok) {
      toast(
        `Failed to verify OTP: ${response.data.error || "Unknown error"}`,
        "error"
      );
      props.closeFn(false);
      return;
    }

    toast("Two-Factor Authentication enabled successfully!", "success");
    props.refetchUserInfo();
    props.closeFn(false);
  };

  createEffect(() => {
    const mfaSecretValue = mfaSecret();
    console.log("MFA Secret on mount:", mfaSecretValue);

    if (mfaSecretValue && mfaSecretValue.secret) {
      console.log("MFA Secret fetched:", mfaSecretValue);

      const otpAuthUrl = mfaSecretValue.secret;
      generateQrCode(otpAuthUrl);
    }
  });

  return (
    <ModalContainer
      width="600px"
      closeFn={props.closeFn}
      class="flex flex-col gap-3 items-center"
    >
      <h2 class="text-2xl font-semibold mb-4 text-primary">
        Secure Your Account
      </h2>
      <p class="text-center text-white">
        Scan the QR code below with your preferred authenticator app and then
        enter the provided one time code below
      </p>

      <div class="border border-border-color p-4 rounded-xs min-h-[264px] min-w-[264px] flex justify-center items-center">
        <img width="auto" src={qrDataUrl() as string} />
      </div>

      <div class="w-full flex items-center gap-2">
        <div class="h-px w-full bg-grey/20"></div>
        <span class="text-white">THEN</span>
        <div class="h-px w-full bg-grey/20"></div>
      </div>

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
        >
          VERIFY
        </Button>
      </form>
    </ModalContainer>
  );
};

export default TwoFactorAuthModal;
