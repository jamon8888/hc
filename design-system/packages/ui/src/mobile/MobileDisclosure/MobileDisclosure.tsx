import { ChevronRight as LucideChevronRight } from 'lucide-react';
import { type HTMLAttributes, type ReactNode } from "react";
import { classNames } from "../../internal/classNames";
import styles from "./MobileDisclosure.module.css";
export interface MobileDisclosureProps extends Omit<HTMLAttributes<HTMLDivElement>, "onToggle" | "title"> { children: ReactNode; disabled?: boolean; leading?: ReactNode; metadata?: ReactNode; onToggle: () => void; open: boolean; title: ReactNode; }
export function MobileDisclosure({ children, className, disabled, leading, metadata, onToggle, open, title, ...props }: MobileDisclosureProps) {
  return (
    <div {...props} className={classNames(styles.root, className)} data-alex-component="mobile-disclosure" data-open={open ? "true" : "false"}>
      <button aria-expanded={open} className={styles.trigger} data-alex-part="trigger" disabled={disabled} onClick={onToggle} type="button">
        {leading && <span className={styles.leading} data-alex-part="leading">{leading}</span>}
        <span className={styles.title} data-alex-part="title">{title}</span>
        {metadata && <span className={styles.metadata} data-alex-part="metadata">{metadata}</span>}
        <LucideChevronRight aria-hidden="true" className={styles.chevron} data-alex-part="chevron" />
      </button>
      {open && <div className={styles.body} data-alex-part="body">{children}</div>}
    </div>
  );
}
