import { createSignal, onMount, onCleanup } from "solid-js";

declare global {
  interface Window {
    turnstile?: {
      render: (
        container: HTMLElement,
        options: {
          sitekey: string;
          callback?: (token: string) => void;
          "expired-callback"?: () => void;
          "error-callback"?: () => void;
          theme?: "light" | "dark" | "auto";
          action?: string;
          size?: "normal" | "compact" | "flexible";
        }
      ) => string;
      reset: (widgetId: string) => void;
      remove: (widgetId: string) => void;
    };
  }
}

interface TurnstileProps {
  onVerify: (token: string) => void;
  onExpire?: () => void;
  onError?: () => void;
  action?: string;
  theme?: "light" | "dark" | "auto";
  size?: "normal" | "compact" | "flexible";
  class?: string;
}

const Turnstile = (props: TurnstileProps) => {
  let containerRef: HTMLDivElement | undefined;
  const [widgetId, setWidgetId] = createSignal<string | null>(null);

  const tryRender = () => {
    if (!containerRef) return;

    if (window.turnstile) {
      const id = window.turnstile.render(containerRef, {
        sitekey: import.meta.env.VITE_TURNSTILE_SITE_KEY,
        callback: (token: string) => props.onVerify(token),
        "expired-callback": () => props.onExpire?.(),
        "error-callback": () => props.onError?.(),
        theme: props.theme ?? "dark",
        action: props.action,
        size: props.size ?? "flexible",
      });
      setWidgetId(id);
    } else {
      // Turnstile script not loaded yet, retry
      setTimeout(tryRender, 100);
    }
  };

  onMount(() => {
    tryRender();
  });

  onCleanup(() => {
    const id = widgetId();
    if (id && window.turnstile) {
      try {
        window.turnstile.remove(id);
      } catch {
        // Widget might already be removed
      }
    }
  });

  return (
    <div class={`${props.class ?? ""} scale-90 origin-center`}>
      <div ref={containerRef} />
    </div>
  );
};

export default Turnstile;
