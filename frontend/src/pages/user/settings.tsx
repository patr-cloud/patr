import { createResource, Suspense } from "solid-js";
import { GetUserInfoResponse } from "~/bindings";
import {
  Button,
  ButtonVariant,
  Input,
  InputLabel,
  InputType,
  Modal,
  PageContainer,
  PageContainerBody,
  PageContainerHead,
  useToast,
} from "~/components";
import { useAuthState } from "~/hooks";
import { httpRequest } from "~/utils/http-request";
import { EventT } from "~/utils/types";
import TwoFactorAuthModal from "./two-fa";

const UserSettingsPage = () => {
  const [authState] = useAuthState();
  const toast = useToast();

  const [userInfo, { mutate: mutateUserInfo, refetch: refetchUserInfo }] =
    createResource(authState(), async (auth) => {
      if (auth === null || auth.type !== "LoggedIn") {
        console.log("Auth is null or LoggedOut, returning null");
        return undefined;
      }

      try {
        const response = await httpRequest<GetUserInfoResponse>(
          `${import.meta.env.VITE_BASE_URL}/api/user`,
          {
            method: "GET",
            headers: {
              "Content-Type": "application/json",
              Authorization: `Bearer ${auth.accessToken}`,
            },
          }
        );

        if (!response.ok) {
          console.error("Failed to fetch workspaces:", response.data.error);
          toast("Failed to fetch workspaces", "error");
          return undefined;
        }

        return response.data;
      } catch (error) {
        console.error("Failed to fetch user info:", error);
        return undefined;
      }
    });

  const onUpdateName = async (e: EventT<SubmitEvent, HTMLFormElement>) => {
    e.preventDefault();
    const auth = authState();

    if (!auth || auth.type !== "LoggedIn") {
      toast("You must be logged in to update your name", "error");
      return;
    }

    try {
      const response = await httpRequest(
        `${import.meta.env.VITE_BASE_URL}/api/user`,
        {
          method: "PATCH",
          headers: {
            "Content-Type": "application/json",
            Authorization: `Bearer ${auth.accessToken}`,
          },
          body: JSON.stringify({
            firstName: userInfo.latest?.firstName,
            lastName: userInfo.latest?.lastName,
          }),
        }
      );

      if (!response.ok) {
        console.error("Failed to update user info:", response.data.error);
        toast("Failed to update user info", "error");
        return;
      }

      toast("User info updated successfully", "success");
      refetchUserInfo();
    } catch (error) {
      console.error("Failed to update user info:", error);
      toast("Failed to update user info", "error");
    }
  };

  return (
    <PageContainer>
      <PageContainerHead title="User" subTitle="Settings" />
      <PageContainerBody class="flex flex-col gap-8">
        <Suspense fallback={<div>Loading user info...</div>}>
          <div class="flex flex-col gap-6 items-start w-full">
            <form
              onSubmit={onUpdateName}
              class="flex gap-4 items-center w-full"
            >
              <InputLabel parentClass="flex-1" for="first-name" label="Name" />
              <Input
                value={userInfo.latest?.firstName || ""}
                class="flex-5"
                name="first-name"
                placeholder="First Name"
                type={InputType.Text}
                onInput={(e) => {
                  mutateUserInfo((prev) => {
                    return prev
                      ? {
                          ...prev,
                          firstName: e.currentTarget.value,
                        }
                      : undefined;
                  });
                }}
              />
              <Input
                value={userInfo.latest?.lastName || ""}
                class="flex-5"
                name="last-name"
                placeholder="Last Name"
                type={InputType.Text}
                onInput={(e) => {
                  mutateUserInfo((prev) => {
                    return prev
                      ? {
                          ...prev,
                          lastName: e.currentTarget.value,
                        }
                      : undefined;
                  });
                }}
              />
              <Button type="submit" variant={ButtonVariant.Contained}>
                UPDATE
              </Button>
            </form>

            <div class="flex gap-4 items-center w-full">
              <InputLabel parentClass="flex-1" for="first-name" label="Email" />

              <Input
                value={userInfo.latest?.recoveryEmail || ""}
                class="flex-11"
                name="recovery-email"
                placeholder="Recovery Email"
                type={InputType.Text}
                disabled
              />
            </div>

            <form class="flex gap-4 items-start w-full">
              <InputLabel
                parentClass="flex-1"
                for="2-fa"
                label="Two-Factor Authentication"
              />

              <div class="flex-11">
                <Modal
                  renderTrigger={(open) => (
                    <Button
                      variant={ButtonVariant.Contained}
                      type="button"
                      class="text-primary"
                      onClick={() => open(true)}
                    >
                      {userInfo.latest?.isMfaEnabled ? "Disable" : "Enable"} 2FA
                      Settings
                    </Button>
                  )}
                  renderModalContent={(close) => (
                    <TwoFactorAuthModal
                      isMfaEnabled={!!userInfo.latest?.isMfaEnabled}
                      refetchUserInfo={refetchUserInfo}
                      closeFn={close}
                    />
                  )}
                />
              </div>
            </form>
          </div>
        </Suspense>
      </PageContainerBody>
    </PageContainer>
  );
};

export default UserSettingsPage;
