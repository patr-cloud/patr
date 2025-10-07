import { JSX } from "solid-js";

export type PropWithChildren<T = {}> = T & {
  children?: JSX.Element;
};
