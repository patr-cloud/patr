import EmailLink from "./link";
import { Hr, Text } from "@react-email/components";

export default function Footer() {
  return (
    <>
      <Hr className="border-secondary-light border-2 my-4" />
      <Text className="px-6 text-white m-0">
        Join our&nbsp;
        <EmailLink href="https://patr.cloud/discord">
          Discord Community
        </EmailLink>
        &nbsp; for the latest updates and chat with us&nbsp;
        <EmailLink href="https://patr.cloud/support">here</EmailLink> for
        instant technical support.
      </Text>
    </>
  );
}
