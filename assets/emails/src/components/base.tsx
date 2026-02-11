import {
  Head,
  Preview,
  Tailwind,
  Body,
  Container,
  Section,
  Font,
} from "@react-email/components";
import { ReactNode } from "react";
import Header from "./header";
import Footer from "./footer";

export type BaseEmailProps = {
  username: string;
  subject: string;
  children: ReactNode;
};

export default function BaseEmail({
  username,
  children,
  subject,
}: BaseEmailProps) {
  return (
    <Tailwind
      config={{
        theme: {
          extend: {
            fontFamily: {
              poppins: [
                "Poppins",
                "Roboto",
                "Helvetica",
                "Arial",
                "sans-serif",
              ],
              primary: [
                "Poppins",
                "Roboto",
                "Helvetica",
                "Arial",
                "sans-serif",
              ],
            },
            colors: {
              primary: {
                DEFAULT: "#f89b41",
                light: "#f89b4180",
                dark: "#f89b41",
              },
              secondary: {
                DEFAULT: "#0d0526",
                dark: "#191131",
                medium: "#2e2450",
                light: "#23203e",
              },
              error: {
                DEFAULT: "#d62b36",
                dark: "#e74c3c",
                light: "#d62b3680",
              },
              warning: {
                DEFAULT: "#fdd13a",
                dark: "#f1c40f",
                light: "#fdd13a80",
              },
              success: {
                DEFAULT: "#47c96c",
                dark: "#07bc0c",
                light: "#47c96c80",
              },
              info: {
                DEFAULT: "#007bff",
                dark: "#3498db",
                light: "#007bff80",
              },
              white: "#ffffff",
              black: "#000000",
              grey: "#ffffffac",
              disabled: "#ffffff60",
              "border-color": "#414245",
            },
            fontSize: {
              xxs: "10px",
              xs: "12px",
              sm: "13px",
              md: "16px",
              lg: "18px",
              xl: "24px",
              xxl: "33px",
            },
            fontWeight: {
              thin: "300",
              regular: "400",
              medium: "500",
              bold: "600",
            },
            borderRadius: {
              xs: "0.25rem",
              sm: "0.625rem",
              DEFAULT: "0.875rem",
              md: "0.875rem",
              lg: "1.125rem",
              xl: "1.25rem",
            },
            boxShadow: {
              high: "4px 4px 10px #000000f0",
              medium: "4px 4px 10px #00000080",
              light: "5px 0px 10px #00000040",
            },
            spacing: {
              xxs: "6px",
              xs: "8px",
              sm: "11px",
              md: "16px",
              lg: "20px",
              xl: "24px",
              xxl: "33px",
            },
          },
        },
      }}
    >
      <Head>
        <meta httpEquiv="Content-Type" content="text/html; charset=utf-8" />
        <meta httpEquiv="X-UA-Compatible" content="IE=edge" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <meta name="color-scheme" content="light only" />
        <meta name="supported-color-schemes" content="light only" />
        <style>{`
          :root { color-scheme: light only; }
        `}</style>
      </Head>

      <Preview>{subject}</Preview>

      <Body className="m-0 p-0 bg-secondary font-poppins">
        <Container className="py-6">
          <Header username={username} />

          <Section className="px-6">{children}</Section>

          <Footer />
        </Container>
      </Body>
    </Tailwind>
  );
}

BaseEmail.PreviewProps = {
  username: "rakshith-ravi",
  subject: "This is a preview of the email subject",
  children: "This is a preview of the email body",
};
