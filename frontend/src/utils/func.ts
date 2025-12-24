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

export { get, Jsx, Uuid, variantBgClass };
