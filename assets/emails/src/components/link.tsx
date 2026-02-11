import { Link } from "@react-email/components";

const EmailLink = ({
  href,
  children,
  className,
}: {
  href: string;
  children: React.ReactNode;
  className?: string;
}) => {
  return (
    <Link
      href={href}
      className={`text-primary no-underline font-medium ${className ?? ""}`}
    >
      {children}
    </Link>
  );
};

export default EmailLink;
