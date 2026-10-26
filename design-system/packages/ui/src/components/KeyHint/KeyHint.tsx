import {
  forwardRef,
  type HTMLAttributes,
  type ReactNode,
} from "react";
import { classNames } from "../../internal/classNames";
import styles from "./KeyHint.module.css";

export interface KeyHintProps extends HTMLAttributes<HTMLElement> {
  icon?: ReactNode;
}

export const KeyHint = forwardRef<HTMLElement, KeyHintProps>(function KeyHint({
  children,
  className,
  icon,
  ...props
}, ref) {
  return (
    <kbd
      {...props}
      className={classNames(styles.root, className)}
      data-alex-component="key-hint"
      ref={ref}
    >
      {icon !== undefined && icon !== null && (
        <span aria-hidden="true" className={styles.icon} data-alex-icon-slot="true" data-alex-part="icon">
          {icon}
        </span>
      )}
      <span className={styles.label} data-alex-part="label">{children}</span>
    </kbd>
  );
});
