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
  return (
    <div class={`${props.class ?? ""} scale-90 origin-center`}>
      <div
        class={`cf-turnstile ${props.class ?? ""}`}
        data-sitekey={import.meta.env.VITE_TURNSTILE_SITE_KEY}
        data-theme={props.theme ?? "dark"}
        data-action={props.action}
        data-size={props.size ?? "flexible"}
        data-callback={(token: string) => props.onVerify(token)}
        data-expired-callback={() => props.onExpire?.()}
        data-error-callback={() => props.onError?.()}
      />
    </div>
  );
};

export default Turnstile;
