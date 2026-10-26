import {
  forwardRef,
  type ButtonHTMLAttributes,
  type ReactNode,
} from "react";
import { classNames } from "../../internal/classNames";
import styles from "./IconButton.module.css";

export interface IconButtonProps
  extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "aria-label" | "children"> {
  "aria-label": string;
  icon: ReactNode;
  loading?: boolean;
  shape?: "circle" | "square";
  size?: "xs" | "standard" | "sm" | "md" | "lg";
  tone?: "danger" | "neutral";
  variant?: "fill" | "outline" | "primary" | "quiet";
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton({
  "aria-label": ariaLabel,
  className,
  disabled,
  icon,
  loading = false,
  shape = "square",
  size = "sm",
  tone = "neutral",
  type = "button",
  variant = "quiet",
  ...props
}, ref) {
  return (
    <button
      {...props}
      aria-busy={loading || undefined}
      aria-label={ariaLabel}
      className={classNames(styles.button, className)}
      data-alex-component="icon-button"
      data-alex-shape={shape}
      data-alex-tone={tone}
      data-alex-variant={variant}
      data-loading={loading ? "true" : "false"}
      data-size={size}
      disabled={disabled || loading}
      ref={ref}
      type={type}
    >
      <span aria-hidden="true" className={styles.progress} data-alex-part="progress" />
      <span aria-hidden="true" className={styles.icon} data-alex-icon-slot="true" data-alex-part="icon">{icon}</span>
    </button>
  );
});
