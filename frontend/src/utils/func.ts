import { Accessor, JSX } from "solid-js";

const get = <T>(v: T | Accessor<T>): T =>
  typeof v === "function" ? (v as Accessor<T>)() : v;

const Jsx = (element: JSX.Element) => {
  return () => element;
};

function Uuid(value: string) {
  return value.replaceAll("-", "");
}

export { get, Jsx, Uuid };
