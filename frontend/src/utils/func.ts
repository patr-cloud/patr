import { Accessor, JSX } from "solid-js";

const get = <T>(v: T | Accessor<T>): T =>
  typeof v === "function" ? (v as Accessor<T>)() : v;

const Jsx = (element: JSX.Element) => {
  return () => element;
};

function Uuid(value: string) {
  return value.replaceAll("-", "");
}

function variantBgClass(styleVariant: string) {
  switch (styleVariant) {
    case "light":
      return "bg-secondary-light";
    case "medium":
      return "bg-secondary-medium";
    case "dark":
      return "bg-secondary-dark";
    default:
      return "bg-secondary-light";
  }
}

// Helper to parse permission names like "deployment::view" into { resourceType: "deployment", action: "view" }
const parsePermissionName = (name: string) => {
  const parts = name.split("::");
  return {
    resourceType: parts[0] || "",
    action: parts[1] || name,
  };
};

const parseCamelCase = (str: string) => {
  return str
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .replace(/^./, (char) => char.toUpperCase());
};

export { get, Jsx, Uuid, variantBgClass, parseCamelCase, parsePermissionName };
