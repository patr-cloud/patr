import { Section, Text } from "@react-email/components";
import EmailButton from "./components/button";
import EmailLink from "./components/link";
import BaseEmail from "./components/base";

export type UserSignUpProps = {
  username: string;
  otp: string;
};

export default function UserSignUp({ username, otp }: UserSignUpProps) {
  const confirmUrl = `https://app.patr.cloud/sign-up/confirm?otp=${encodeURIComponent(
    otp,
  )}&username=${encodeURIComponent(username)}`;

  return (
    <BaseEmail username={username} subject="Confirm your sign up | Patr">
      <Section className="bg-secondary">
        <Text className="text-white text-md pb-4">
          Thanks for signing up for Patr! To complete your sign-up process,
          please verify your email address by entering the one-time code below:
        </Text>

        <Section className="text-center w-full border-2 border-secondary-light rounded-xs">
          <Text className="text-primary text-5xl font-extrabold">{otp}</Text>
        </Section>

        <Text className="text-white pt-4">
          Or just click the button below and we'll verify it automatically:
        </Text>

        <Section className="text-center">
          <EmailButton href={confirmUrl}>Verify Email</EmailButton>
        </Section>

        <Text className="pt-4 text-white">
          If you did not initiate email verification on Patr with this email
          address, kindly ignore this email and reach out to us at{" "}
          <EmailLink href="mailto:support@patr.cloud">
            support@patr.cloud
          </EmailLink>
          .
        </Text>
      </Section>
    </BaseEmail>
  );
}

UserSignUp.PreviewProps = {
  username: "rakshith-ravi",
  otp: "123-456",
};
