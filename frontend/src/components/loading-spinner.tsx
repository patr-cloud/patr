import { mergeProps } from "solid-js";

interface LoadingSpinnerProps {
  /**
   * Size of the spinner
   */
  size?: "sm" | "md" | "lg";
  /**
   * Additional CSS classes
   */
  class?: string;
}

const LoadingSpinner = (rawProps: LoadingSpinnerProps) => {
  const props = mergeProps(
    {
      size: "md" as const,
      class: "",
    },
    rawProps
  );

  const sizeClasses = () => {
    switch (props.size) {
      case "sm":
        return "w-4 h-4 border-2";
      case "md":
        return "w-6 h-6 border-2";
      case "lg":
        return "w-8 h-8 border-[3px]";
      default:
        return "w-6 h-6 border-2";
    }
  };

  return (
    <div
      class={`${sizeClasses()} border-primary border-t-transparent rounded-full animate-spin ${props.class}`}
      role="status"
      aria-label="Loading"
    >
      <span class="sr-only">Loading...</span>
    </div>
  );
};

export default LoadingSpinner;