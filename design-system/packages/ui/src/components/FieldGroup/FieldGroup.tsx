import {
  createElement,
  forwardRef,
  type HTMLAttributes,
  type ReactNode,
} from "react";
import { classNames } from "../../internal/classNames";
import { FieldSurfaceContext, type FieldSurface } from "../../internal/fieldSurface";
import styles from "./FieldGroup.module.css";

export type FieldGroupAppearance = "plain" | "subtle";
export type FieldGroupFieldSurface = FieldSurface;
export type FieldRowAlignment = "center" | "start";
export type FieldRowPadding = "none" | "md";
export type FormSectionHeading = "h2" | "h3" | "h4";

export interface FormSectionProps
  extends Omit<HTMLAttributes<HTMLElement>, "title"> {
  actions?: ReactNode;
  description?: ReactNode;
  headingAs?: FormSectionHeading;
  leading?: ReactNode;
  title?: ReactNode;
}

export interface FieldGroupProps extends HTMLAttributes<HTMLDivElement> {
  appearance?: FieldGroupAppearance;
  dividers?: boolean;
  /** Lets text and picker field shells reuse the surrounding grouped surface. */
  fieldSurface?: FieldGroupFieldSurface;
}

export interface FieldRowProps extends HTMLAttributes<HTMLDivElement> {
  align?: FieldRowAlignment;
  padding?: FieldRowPadding;
}

export const FormSection = forwardRef<HTMLElement, FormSectionProps>(
  function FormSection({
    actions,
    children,
    className,
    description,
    headingAs = "h2",
    leading,
    title,
    ...props
  }, ref) {
    return (
      <section
        {...props}
        className={classNames(styles.section, className)}
        data-alex-component="form-section"
        ref={ref}
      >
        {(title !== undefined && title !== null)
          || (description !== undefined && description !== null)
          || (leading !== undefined && leading !== null)
          || (actions !== undefined && actions !== null) ? (
            <div className={styles.header} data-alex-part="header">
              {(title !== undefined && title !== null)
                || (description !== undefined && description !== null)
                || (leading !== undefined && leading !== null) ? (
                  <div className={styles.headingRegion} data-alex-part="heading-region">
                    {leading !== undefined && leading !== null && (
                      <div className={styles.leading} data-alex-part="leading">{leading}</div>
                    )}
                    {(title !== undefined && title !== null)
                      || (description !== undefined && description !== null) ? (
                        <div className={styles.headingContent} data-alex-part="heading-content">
                          {title !== undefined && title !== null && createElement(
                            headingAs,
                            { className: styles.title, "data-alex-part": "title" },
                            title,
                          )}
                          {description !== undefined && description !== null && (
                            <div className={styles.description} data-alex-part="description">
                              {description}
                            </div>
                          )}
                        </div>
                      ) : null}
                  </div>
                ) : null}
              {actions !== undefined && actions !== null && (
                <div className={styles.actions} data-alex-part="actions">{actions}</div>
              )}
            </div>
          ) : null}
        {children}
      </section>
    );
  },
);

export const FieldGroup = forwardRef<HTMLDivElement, FieldGroupProps>(
  function FieldGroup({
    appearance = "subtle",
    children,
    className,
    dividers = true,
    fieldSurface = "default",
    ...props
  }, ref) {
    return (
      <FieldSurfaceContext.Provider value={fieldSurface}>
        <div
          {...props}
          className={classNames(styles.group, className)}
          data-appearance={appearance}
          data-alex-component="field-group"
          data-dividers={dividers ? "true" : "false"}
          data-field-surface={fieldSurface}
          ref={ref}
        >
          {children}
        </div>
      </FieldSurfaceContext.Provider>
    );
  },
);

export const FieldRow = forwardRef<HTMLDivElement, FieldRowProps>(
  function FieldRow({
    align = "center",
    children,
    className,
    padding = "md",
    ...props
  }, ref) {
    return (
      <div
        {...props}
        className={classNames(styles.row, className)}
        data-align={align}
        data-alex-part="row"
        data-padding={padding}
        ref={ref}
      >
        {children}
      </div>
    );
  },
);
