import { Portal } from "solid-js/web";
import {
  createContext,
  createEffect,
  For,
  onCleanup,
  ParentProps,
  splitProps,
  useContext,
} from "solid-js";
import { createStore, SetStoreFunction } from "solid-js/store";
import { FiAlertCircle, FiCheckCircle } from "solid-icons/fi";

export interface ToastData {
  id: number;
  level: "warn" | "error" | "success";
  expiry: number;
  dismissible: boolean;
  message: string;
}

const Toaster = (props: {
  toasts: ToastData[];
  setToasts: SetStoreFunction<ToastData[]>;
}) => {
  return (
    <Portal>
      <div class="fixed top-4 right-4 flex flex-col gap-2 z-51">
        <For each={props.toasts}>
          {(toast) => <Toast toast={toast} setToasts={props.setToasts} />}
        </For>
      </div>
    </Portal>
  );
};

const Toast = (props: {
  toast: ToastData;
  setToasts: SetStoreFunction<ToastData[]>;
}) => {
  const [{ toast, setToasts }] = splitProps(props, ["toast", "setToasts"]);

  const handleClick = () => {
    if (toast.dismissible) {
      setToasts((prev) => prev.filter((t) => t.id !== props.toast.id));
    }
  };

  createEffect(() => {
    if (toast.expiry > 0) {
      const timeout = setTimeout(() => {
        setToasts((prev) => prev.filter((t) => t.id !== toast.id));
      }, toast.expiry);

      onCleanup(() => {
        clearTimeout(timeout);
      });
    }
  });

  let backgroundColor: string;
  switch (toast.level) {
    case "success":
      backgroundColor = "bg-success";
      break;
    case "error":
      backgroundColor = "bg-error";
      break;
    case "warn":
      backgroundColor = "bg-warning";
      break;
    default:
      backgroundColor = "bg-info";
  }

  return (
    <div
      class={`${backgroundColor} text-white rounded-xs min-h-16 min-w-60 flex items-center justify-start gap-2 max-h-12 p-sm text-sm ${
        toast.dismissible ? "cursor-pointer" : ""
      }`}
      onClick={handleClick}
    >
      {toast.level === "success" && (
        <FiCheckCircle class="text-success-dark" size={20} />
      )}
      {toast.level === "warn" && (
        <FiCheckCircle class="text-warning-dark" size={20} />
      )}
      {toast.level === "error" && (
        <FiAlertCircle class="text-error-dark" size={20} />
      )}
      {toast.message}
    </div>
  );
};

type CreateToastFn = (
  /** The message to display in the toast */
  message: string,
  /** The level of the toast */
  level: "warn" | "error" | "success",
  /** Whether the toast can be dismissed by clicking on it */
  dismissible?: boolean,
  /** The time in milliseconds before the toast expires, set to -1 to disable expiry */
  expiry?: number
) => void;

type ToastContextType = [
  ToastData[],
  {
    createToast: CreateToastFn;
    removeToast: (id: number) => void;
    clear: () => void;
  }
];

const ToastContext = createContext<ToastContextType | null>(null);

const ToastProvider = (props: ParentProps<{}>) => {
  const [toasts, setToasts] = createStore<ToastData[]>([]);

  const createToast: CreateToastFn = (
    message,
    level,
    dismissible = true,
    expiry = 5000
  ) => {
    const id = Date.now();

    if (toasts.length >= 3) {
      setToasts((prev) => prev.slice(1));
    }

    const newToast: ToastData = {
      id,
      level,
      message,
      dismissible,
      expiry,
    };
    setToasts((prev) => [...prev, newToast]);
  };

  const removeToast = (id: number) => {
    setToasts((prev) => prev.filter((toast) => toast.id !== id));
  };

  const clear = () => {
    setToasts([]);
  };

  const value: ToastContextType = [
    toasts,
    {
      createToast,
      removeToast,
      clear,
    },
  ];

  return (
    <ToastContext.Provider value={value}>
      {props.children}
      <Toaster toasts={toasts} setToasts={setToasts} />
    </ToastContext.Provider>
  );
};

/**
 * Hook to get the create toast function, must be used within a ToastProvider
 * To get the remove, clear, or the toast array, use the ToastContext directly
 *
 * @throws {Error} If used outside of a ToastProvider
 * @example
 * const toast = useToast();
 * toast("success", "This is a success message", true, 3000);
 */
function useToast(): CreateToastFn {
  const toast = useContext(ToastContext);

  if (!toast) {
    throw new Error("useToast must be used within a ToastProvider");
  }

  // This is the create toast function
  return toast[1].createToast;
}

export { ToastProvider, useToast };
