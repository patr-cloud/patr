import { Img, Text } from "@react-email/components";

const baseUrl = process.env.VERCEL_URL
  ? `https://${process.env.VERCEL_URL}`
  : "";

export default function Header({ username }: { username: string }) {
  return (
    <>
      <Img
        src={`${baseUrl}/static/images/header.png`}
        alt="Patr Logo"
        className="w-[100%] mb-6"
      />
      <Text className="text-white font-bold text-lg px-6">
        Hello {username || "rakshith-ravi"},
      </Text>
    </>
  );
}

Header.PreviewProps = {
  username: "rakshith-ravi",
};
