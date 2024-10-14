/** @type {import('tailwindcss').Config} */
export default {
  content: [
    ".html",
    "frontend/src/**/*.rs",
    "components/src/**/*.rs",
  ],
  corePlugins: {
    preflight: false,
  },
  theme: {
    extend: {
      colors: {
        primary: "#f89b41",
        "primary-light": "#f89b4180",
        "primary-dark": "#f89b41",
        "secondary-dark": "#191131",
        secondary: "#0d0526",
        "secondary-medium": "#2e2450",
        "secondary-light": "#23203e",

        error: "#d62b36",
        "error-light": "#d62b3680",
        warning: "#fdd13a",
        "warning-light": "#fdd13a80",
        success: "#47c96c",
        "success-light": "#47c96c80",
        info: "#007bff",
        "info-light": "#007bff80",

        white: "#ffffff",
        black: "#000000",
        grey: "#ffffffac",
        disabled: "#ffffff60",

        "txt-code-snippet": "#a5c5ff",
        "bg-code-snippet": "#78a7ff1a",

        "border-color": "#414245",

        "tooltip-color": "#333333",
      },
      fontFamily: {
        primary: ["Poppins", "Roboto", "Helvetica", "Arial", "sans-serif"],
        log: ["Source Code Pro", "monospace"],
      },
      fontWeight: {
        thin: "300",
        regular: "400",
        medium: "500",
        bold: "600",
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
      boxShadow: {
        high: "4px 4px 10px #000000f0",
        medium: "4px 4px 10px #00000080",
        light: "5px 0px 10px #00000040",
      },
      borderRadius: {
        sm: "10px",
        DEFAULT: "14px",
        md: "14px",
        lg: "18px",
        xl: "20px",
      },
      spacing: {
        xxs: "6px",
        xs: "8px",
        sm: "11px",
        md: "16px",
        lg: "20px",
        xl: "24px",
        xxl: "33px",
        auto: "auto",
      },
      flex: {
        1: "0 1 8.3333%",
        2: "0 1 16.6666%",
        3: "0 1 25%",
        4: "0 1 33.3333%",
        5: "0 1 41.6666%",
        6: "0 1 50%",
        7: "0 1 58.3333%",
        8: "0 1 66.6666%",
        9: "0 1 75%",
        10: "0 1 83.333%",
        11: "0 1 91.6666%",
        12: "0 1 100%",
      },
    },
  },
  plugins: [],
};
