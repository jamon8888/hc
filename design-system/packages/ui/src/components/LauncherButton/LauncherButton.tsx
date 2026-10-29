import {
  forwardRef,
  type ButtonHTMLAttributes,
  type ReactNode,
} from "react";
import { classNames } from "../../internal/classNames";
import { OverflowText } from "../../primitives/OverflowText";
import styles from "./LauncherButton.module.css";

export interface LauncherButtonProps
  extends ButtonHTMLAttributes<HTMLButtonElement> {
  children: ReactNode;
  leadingIcon?: ReactNode;
}

export const LauncherButton = forwardRef<
  HTMLButtonElement,
  LauncherButtonProps
>(function LauncherButton({
  children,
  className,
  leadingIcon,
  type = "button",
  ...props
}, ref) {
  return (
    <button data-overflow-trigger
      {...props}
      className={classNames(styles.root, className)}
      data-alex-component="launcher-button"
      data-alex-part="root"
      ref={ref}
      type={type}
    >
      {leadingIcon !== undefined && leadingIcon !== null && (
        <span aria-hidden="true" className={styles.icon} data-alex-icon-slot="true" data-alex-part="icon">
          {leadingIcon}
        </span>
      )}
      <OverflowText className={styles.label} data-alex-part="label">
        {children}
      </OverflowText>
    </button>
  );
});
