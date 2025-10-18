import { Accessor, JSX } from "solid-js";

export type PropWithChildren<T = {}> = T & {
  children?: JSX.Element;
};

export type MaybeAccessor<T> = T | Accessor<T>;
