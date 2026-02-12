import { Img, Text } from "@react-email/components";

export default function Header({ username }: { username: string }) {
  return (
    <>
      <Img
        src={`/static/header.png`}
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
