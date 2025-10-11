import { createSignal, Show, mergeProps } from "solid-js";
import { JSX } from "solid-js/h/jsx-runtime";

interface ErrorMessageProps {
  /**
   * Error message to display
   */
  message: string;
  /**
   * Whether the error message can be dismissed
   */
  dismissible?: boolean;
  /**
   * Whether to show a retry button
   */
  showRetry?: boolean;
  /**
   * Callback function for retry action
   */
  onRetry?: () => void;
  /**
   * Callback function when message is dismissed
   */
  onDismiss?: () => void;
  /**
   * Additional CSS classes
   */
  class?: string;
}

const ErrorMessage = (rawProps: ErrorMessageProps) => {
  const props = mergeProps(
    {
      dismissible: false,
      showRetry: false,
      class: "",
    },
    rawProps
  );

  const [isVisible, setIsVisible] = createSignal(true);

  const handleDismiss = () => {
    setIsVisible(false);
    props.onDismiss?.();
  };

  const handleRetry = () => {
    props.onRetry?.();
  };

  return (
    <Show when={isVisible()}>
      <div
        class={`bg-error-light border border-error rounded-xs p-md flex items-start gap-sm ${props.class}`}
        role="alert"
      >
        {/* Error Icon */}
        <div class="flex-shrink-0 mt-0.5">
          <svg
            class="w-4 h-4 text-error"
            fill="currentColor"
            viewBox="0 0 20 20"
            xmlns="http://www.w3.org/2000/svg"
          >
            <path
              fill-rule="evenodd"
              d="M10 18a8 8 0 100-16 8 8 0 000 16zM8.707 7.293a1 1 0 00-1.414 1.414L8.586 10l-1.293 1.293a1 1 0 101.414 1.414L10 11.414l1.293 1.293a1 1 0 001.414-1.414L11.414 10l1.293-1.293a1 1 0 00-1.414-1.414L10 8.586 8.707 7.293z"
              clip-rule="evenodd"
            />
          </svg>
        </div>

        {/* Error Message */}
        <div class="flex-1">
          <p class="text-sm text-error font-medium">{props.message}</p>
        </div>

        {/* Action Buttons */}
        <div class="flex items-center gap-xs">
          <Show when={props.showRetry}>
            <button
              onClick={handleRetry}
              class="text-xs text-error hover:text-error-light font-medium underline focus:outline-none focus:ring-2 focus:ring-error focus:ring-offset-2 focus:ring-offset-transparent rounded-xs px-xs py-xxs"
            >
              Retry
            </button>
          </Show>

          <Show when={props.dismissible}>
            <button
              onClick={handleDismiss}
              class="flex-shrink-0 text-error hover:text-error-light focus:outline-none focus:ring-2 focus:ring-error focus:ring-offset-2 focus:ring-offset-transparent rounded-xs p-xxs"
              aria-label="Dismiss error"
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
      </div>
    </Show>
  );
};

export default ErrorMessage;