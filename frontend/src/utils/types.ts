import { Accessor } from "solid-js";

export type MaybeAccessor<T> = T | Accessor<T>;
export type EventT<T, E> = T & { currentTarget: E };
export type ErrorResponse = {
  message: string;
  success: false;
  error: string;
};

export type FetchResult<T> =
  | {
      ok: false;
      status: number;
      statusText: string;
      headers: Headers;
      data: ErrorResponse;
    }
  | {
      ok: true;
      status: number;
      statusText: string;
      headers: Headers;
      data: T;
    };
