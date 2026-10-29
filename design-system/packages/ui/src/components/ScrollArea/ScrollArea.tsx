import { forwardRef, type HTMLAttributes } from "react";
import { classNames } from "../../internal/classNames";
import styles from "./ScrollArea.module.css";

export type ScrollAreaOrientation = "vertical" | "horizontal" | "both";
export type ScrollbarVisibility = "auto" | "always" | "hidden";

export interface ScrollAreaProps extends HTMLAttributes<HTMLDivElement> {
  "data-alex-component"?: string;
  "data-alex-part"?: string;
  orientation?: ScrollAreaOrientation;
  /** Auto reveals on viewport hover or keyboard focus; touch remains visible. */
  scrollbarVisibility?: ScrollbarVisibility;
}

export const ScrollArea = forwardRef<HTMLDivElement, ScrollAreaProps>(
  function ScrollArea({
    className,
    "data-alex-component": component = "scroll-area",
    "data-alex-part": part = "viewport",
    orientation = "vertical",
    scrollbarVisibility = "auto",
    ...props
  }, ref) {
    return (
      <div
        {...props}
        className={classNames(styles.root, className)}
        data-alex-component={component}
        data-alex-orientation={orientation}
        data-alex-part={part}
        data-alex-scrollbar-visibility={scrollbarVisibility}
        ref={ref}
      />
    );
  },
);
