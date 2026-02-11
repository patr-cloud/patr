import { ReactNode } from "react";
import { Button } from "@react-email/components";

const EmailButton = ({
  href,
  children,
}: {
  href: string;
  children?: ReactNode;
}) => {
  return (
    <Button
      href={href}
      className={`
        py-xs px-md
        text-secondary bg-primary
        rounded-xs no-underline
        font-thin border-2
        hover:cursor-pointer hover:bg-transparent hover:bg-primary hover:text-primary
        transition-all duration-200
      `}
    >
      {children}
    </Button>
  );
};

export default EmailButton;
