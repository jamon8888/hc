import assert from "node:assert/strict";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { Button } from "../dist/index.js";

test("Button exposes the neutral outline contract by default", () => {
  const markup = renderToStaticMarkup(createElement(Button, null, "Session"));

  assert.match(markup, /data-alex-component="button"/);
  assert.match(markup, /data-alex-tone="neutral"/);
  assert.match(markup, /data-alex-variant="outline"/);
  assert.match(markup, /data-size="md"/);
  assert.match(markup, /type="button"/);
  assert.match(markup, /data-label-behavior="overflow"/);
  assert.match(markup, /data-overflow-behavior="marquee"/);
  assert.match(markup, />Session<\/span>/);
});

test("static labels retain native button semantics without adding overflow machinery", () => {
  const markup = renderToStaticMarkup(createElement(Button, {
    labelBehavior: "static", type: "submit", name: "action", value: "history",
    title: "History details", leadingIcon: createElement("svg"),
  }, "Show previous attempts"));
  assert.match(markup, /type="submit"/);
  assert.match(markup, /name="action"/);
  assert.match(markup, /value="history"/);
  assert.match(markup, /title="History details"/);
  assert.match(markup, /data-alex-part="label">Show previous attempts<\/span>/);
  assert.match(markup, /data-alex-part="leading-icon"/);
  assert.doesNotMatch(markup, /data-overflow-behavior|data-overflow-content|data-overflow-tooltip/);
  const pending = renderToStaticMarkup(createElement(Button, {
    labelBehavior: "static", loading: true,
  }, "Saving"));
  assert.match(pending, /disabled=""/);
  assert.match(pending, /aria-busy="true"/);
  assert.match(pending, />Saving<\/span>/);
});

test("danger tone preserves destructive semantics independently from presentation", async () => {
  const markup = renderToStaticMarkup(
    createElement(Button, { tone: "danger", variant: "fill" }, "Delete"),
  );
  const styles = await readFile(new URL("../dist/styles.css", import.meta.url), "utf8");

  assert.match(markup, /data-alex-tone="danger"/);
  assert.match(markup, /data-alex-variant="fill"/);
  assert.match(styles, /\[data-alex-tone=danger\]/);
  assert.match(styles, /--alex-color-status-danger-content/);
  assert.match(styles, /--alex-color-status-danger-surface/);
  assert.match(styles, /--alex-color-status-danger-border/);
});

test("Button renders decorative icon slots and native disabled state", () => {
  const markup = renderToStaticMarkup(
    createElement(
      Button,
      {
        disabled: true,
        leadingIcon: createElement("svg", { "data-icon": "session" }),
        trailingIcon: createElement("svg", { "data-icon": "chevron-down" }),
        variant: "fill",
      },
      "Session",
    ),
  );

  assert.match(markup, /data-alex-variant="fill"/);
  assert.match(markup, /disabled=""/);
  assert.match(markup, /data-icon="session"/);
  assert.match(markup, /data-icon="chevron-down"/);
  assert.equal((markup.match(/aria-hidden="true"/g) ?? []).length >= 3, true);
});

test("loading keeps the accessible label while disabling activation", () => {
  const markup = renderToStaticMarkup(
    createElement(Button, { loading: true }, "Saving"),
  );

  assert.match(markup, /aria-busy="true"/);
  assert.match(markup, /data-loading="true"/);
  assert.match(markup, /disabled=""/);
  assert.match(markup, />Saving<\/span>/);
});

test("fill styles bind to the public variant attribute", async () => {
  const styles = await readFile(new URL("../dist/styles.css", import.meta.url), "utf8");

  assert.match(styles, /\[data-alex-variant=fill\]/);
  assert.doesNotMatch(styles, /\[data-variant=fill\]/);
});

test("primary and text variants expose semantic emphasis without changing button anatomy", async () => {
  const primaryMarkup = renderToStaticMarkup(
    createElement(Button, { variant: "primary" }, "Save"),
  );
  const textMarkup = renderToStaticMarkup(
    createElement(Button, { variant: "text" }, "Learn more"),
  );
  const styles = await readFile(new URL("../dist/styles.css", import.meta.url), "utf8");

  assert.match(primaryMarkup, /data-alex-variant="primary"/);
  assert.match(textMarkup, /data-alex-variant="text"/);
  assert.match(styles, /--alex-component-button-primary-background/);
  assert.match(styles, /--alex-color-action-primary-content/);
  assert.match(styles, /--alex-component-button-text-content/);
  assert.match(styles, /--alex-component-button-text-content-disabled/);
  assert.match(styles, /text-decoration:underline/);
});

test("secondary exposes the filled secondary-action contract", async () => {
  const markup = renderToStaticMarkup(
    createElement(Button, { variant: "secondary" }, "Cancel"),
  );
  const styles = await readFile(new URL("../dist/styles.css", import.meta.url), "utf8");

  assert.match(markup, /data-alex-variant="secondary"/);
  assert.match(styles, /--alex-color-surface-tertiary/);
  assert.match(styles, /--alex-color-border-default/);
  assert.match(styles, /--alex-color-border-strong/);
  assert.match(styles, /--alex-color-action-neutral-content/);
});

test("outline and text composite over the caller surface without changing their hit targets", async () => {
  const styles = await readFile(
    new URL("../src/components/Button/Button.module.css", import.meta.url),
    "utf8",
  );

  assert.match(
    styles,
    /\.button\s*\{[^}]*background:\s*transparent/s,
  );
  assert.match(
    styles,
    /\.button::before\s*\{[^}]*background:\s*var\(--_button-background\)/s,
  );
  assert.match(
    styles,
    /\[data-alex-variant="text"\]\s*\{[^}]*--_button-background:\s*transparent/s,
  );
  const textRule = styles.match(/\[data-alex-variant="text"\]\s*\{([^}]+)\}/)?.[1] ?? "";
  assert.match(textRule, /--_button-background-hover:\s*transparent/);
  assert.match(textRule, /--_button-background-active:\s*transparent/);
  assert.match(textRule, /border-radius:\s*0/);
  assert.doesNotMatch(textRule, /(?:block-size|padding|font-size|icon-size):/);
  assert.doesNotMatch(styles, /\[data-alex-variant="text"\][^}]*padding-inline:\s*0/s);
});

test("Button owns the reference pill geometry and typography", async () => {
  const styles = await readFile(new URL("../dist/styles.css", import.meta.url), "utf8");

  assert.match(styles, /letter-spacing:var\(--alex-type-label-md-letter-spacing\)/);
  assert.match(styles, /border-radius:var\(--alex-radius-pill\)/);
  assert.match(styles, /--_button-height:\s*var\(--alex-control-height-md\)/);
  assert.match(styles, /--_button-leading-icon-size:\s*16px/);
  assert.match(styles, /--_button-trailing-icon-size:\s*14px/);
  assert.match(styles, /--_button-font-size:\s*var\(--alex-type-label-md-font-size\)/);
  assert.match(styles, /font-family:var\(--alex-type-label-md-font-family\)/);
  assert.match(styles, /font-weight:var\(--alex-type-label-md-font-weight\)/);
  assert.match(styles, /padding-block:var\(--_button-padding-block\)/);
  assert.match(styles, /padding-inline:var\(--_button-padding-inline\)/);
  assert.match(styles, /--_button-padding-inline:\s*var\(--alex-space-5\)/);
  assert.match(
    styles,
    /\[data-size=xs\]\{[^}]*--_button-height:\s*var\(--alex-control-button-xs-height\)/,
  );
  assert.match(
    styles,
    /\[data-size=xs\]\{[^}]*--_button-padding-inline:\s*var\(--alex-control-button-xs-padding-inline\)/,
  );
  assert.match(
    styles,
    /\[data-size=sm\]\{[^}]*--_button-padding-inline:\s*var\(--alex-space-4\)/,
  );
  assert.match(
    styles,
    /\[data-size=lg\]\{[^}]*--_button-padding-inline:\s*var\(--alex-space-6\)/,
  );
  assert.match(
    styles,
    /\[data-size=lg\]\{[^}]*--_button-leading-icon-size:\s*20px/,
  );
  assert.match(styles, /_progress_[^{]+\{[^}]*inline-size:\s*16px;block-size:\s*16px/);
});

test("Button keeps mixed child content vertically centered", async () => {
  const styles = await readFile(
    new URL("../src/components/Button/Button.module.css", import.meta.url),
    "utf8",
  );

  assert.match(styles, /\.label\s*\{[^}]*display:\s*inline-flex/);
  assert.match(styles, /\.label\s*\{[^}]*align-items:\s*center/);
  assert.match(styles, /\.label\s*\{[^}]*gap:\s*var\(--alex-space-1\)/);
  assert.match(styles, /\.label\s*>\s*:where\(svg,\s*img\)\s*\{[^}]*display:\s*block/);
});

test("real and preview hover states share the component rule", async () => {
  const styles = await readFile(new URL("../dist/styles.css", import.meta.url), "utf8");

  assert.match(
    styles,
    /:is\(:hover,\s*\[data-alex-preview-state=(?:"hover"|hover)\]\):not\(:disabled\)/,
  );
});

test("real and preview active states share the semibold component rule", async () => {
  const styles = await readFile(
    new URL("../src/components/Button/Button.module.css", import.meta.url),
    "utf8",
  );

  assert.match(
    styles,
    /:is\(:active,\s*\[data-alex-preview-state=(?:"active"|active)\]\):not\(:disabled\)/,
  );
  assert.match(styles, /font-weight:\s*var\(--alex-type-label-selected-font-weight\)/);
  assert.equal((styles.match(/--alex-type-label-selected-font-weight/g) ?? []).length, 1);
});

test("fill uses Button state colors and icons inherit content color", async () => {
  const styles = await readFile(new URL("../dist/styles.css", import.meta.url), "utf8");

  assert.match(styles, /--_button-background:\s*var\(--alex-component-button-fill-background\)/);
  assert.match(styles, /--_button-background-hover:\s*var\(--alex-component-button-fill-background-hover\)/);
  assert.match(styles, /--_button-background-active:\s*var\(--alex-component-button-fill-background-pressed\)/);
  assert.match(styles, /color:currentColor/);
});

test("native disabled and loading use variant-specific disabled content with a neutral danger fallback", async () => {
  const styles = await readFile(new URL("../src/components/Button/Button.module.css", import.meta.url), "utf8");
  assert.match(styles, /\.button:disabled\s*\{[^}]*color:\s*var\(--_button-content-disabled\)/);
  for (const variant of ["primary", "text"]) {
    const rule = styles.match(new RegExp(`\\[data-alex-variant="${variant}"\\]\\s*\\{([^}]+)\\}`))?.[1] ?? "";
    assert.match(rule, new RegExp(`--_button-content-disabled:\\s*var\\(--alex-component-button-${variant}-content-disabled\\)`));
    const markup = renderToStaticMarkup(createElement(Button, { variant, loading: true }, "Save"));
    assert.match(markup, /disabled=""/);
    assert.match(markup, /aria-busy="true"/);
  }
  assert.match(styles, /\[data-alex-tone="danger"\]\s*\{[^}]*--_button-content-disabled:\s*var\(--alex-color-action-neutral-content-disabled\)/);
});
