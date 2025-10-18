import { Accessor } from "solid-js";

const get = <T>(v: T | Accessor<T>): T =>
  typeof v === "function" ? (v as Accessor<T>)() : v;

export default get;
