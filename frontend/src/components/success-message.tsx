import { createSignal, Show, mergeProps, onMount, onCleanup } from "solid-js";

interface SuccessMessageProps {
  /**
   * Success message to display
   */
  message: string;
  /**
   * Auto-dismiss timeout in milliseconds (default: 5000ms)
   */
  autoHideDelay?: number;
  /**
   * Whether the success message can be manually dismissed
   */
  dismissible?: boolean;
  /**
   * Callback function when message is dismissed
   */
  onDismiss?: () => void;
  /**
   * Additional CSS classes
   */
  class?: string;
}

const SuccessMessage = (rawProps: SuccessMessageProps) => {
  const props = mergeProps(
    {
      autoHideDelay: 5000,
      dismissible: true,
      class: "",
    },
    rawProps
  );

  const [isVisible, setIsVisible] = createSignal(true);
  let timeoutId: number | undefined;

  const handleDismiss = () => {
    setIsVisible(false);
    props.onDismiss?.();
    if (timeoutId) {
      clearTimeout(timeoutId);
    }
  };

  onMount(() => {
    if (props.autoHideDelay > 0) {
      timeoutId = setTimeout(() => {
        handleDismiss();
      }, props.autoHideDelay);
    }
  });

  onCleanup(() => {
    if (timeoutId) {
      clearTimeout(timeoutId);
    }
  });

  return (
    <Show when={isVisible()}>
      <div
        class={`bg-success-light border border-success rounded-xs p-md flex items-start gap-sm ${props.class}`}
        role="alert"
      >
        {/* Success Icon */}
        <div class="flex-shrink-0 mt-0.5">
          <svg
            class="w-4 h-4 text-success"
            fill="currentColor"
            viewBox="0 0 20 20"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              fill-rule="evenodd"
              d="M10 18a8 8 0 100-16 8 8 0 000 16zm3.707-9.293a1 1 0 00-1.414-1.414L9 10.586 7.707 9.293a1 1 0 00-1.414 1.414l2 2a1 1 0 001.414 0l4-4z"
              clip-rule="evenodd"
            />
          </svg>
        </div>

        {/* Success Message */}
        <div class="flex-1">
          <p class="text-sm text-success font-medium">{props.message}</p>
        </div>

        {/* Dismiss Button */}
        <Show when={props.dismissible}>
          <button
            onClick={handleDismiss}
            class="flex-shrink-0 text-success hover:text-success-light focus:outline-none focus:ring-2 focus:ring-success focus:ring-offset-2 focus:ring-offset-transparent rounded-xs p-xxs"
            aria-label="Dismiss success message"
          >
            <svg
              class="w-4 h-4"
              fill="currentColor"
              viewBox="0 0 20 20"
              xmlns="http://www.w3.org/2000/svg"
            >
              <path
                fill-rule="evenodd"
                d="M4.293 4.293a1 1 0 011.414 0L10 8.586l4.293-4.293a1 1 0 111.414 1.414L11.414 10l4.293 4.293a1 1 0 01-1.414 1.414L10 11.414l-4.293 4.293a1 1 0 01-1.414-1.414L8.586 10 4.293 5.707a1 1 0 010-1.414z"
                clip-rule="evenodd"
              />
            </svg>
          </button>
        </Show>
      </div>
    </Show>
  );
};

export default SuccessMessage;