import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";
import { createElement } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ScrollArea } from "../dist/index.js";

test("ScrollArea defaults to a native vertical viewport with automatic visibility", () => {
  const markup = renderToStaticMarkup(
    createElement(ScrollArea, { "aria-label": "Activity" }, "Content"),
  );

  assert.match(markup, /data-alex-component="scroll-area"/);
  assert.match(markup, /data-alex-part="viewport"/);
  assert.match(markup, /data-alex-orientation="vertical"/);
  assert.match(markup, /data-alex-scrollbar-visibility="auto"/);
  assert.match(markup, /aria-label="Activity"/);
});

test("ScrollArea exposes orientation and scrollbar visibility contracts", () => {
  const markup = renderToStaticMarkup(
    createElement(
      ScrollArea,
      { orientation: "both", scrollbarVisibility: "always" },
      "Content",
    ),
  );

  assert.match(markup, /data-alex-orientation="both"/);
  assert.match(markup, /data-alex-scrollbar-visibility="always"/);
});

test("ScrollArea preserves feature-owned appearance contracts", () => {
  const markup = renderToStaticMarkup(
    createElement(
      ScrollArea,
      { "data-alex-component": "model-settings", "data-alex-part": "root" },
      "Content",
    ),
  );

  assert.match(markup, /data-alex-component="model-settings"/);
  assert.match(markup, /data-alex-part="root"/);
});

test("ScrollArea styling uses public scrollbar tokens and preserves native scrolling", async () => {
  const styles = await readFile(
    new URL("../src/components/ScrollArea/ScrollArea.module.css", import.meta.url),
    "utf8",
  );

  assert.match(styles, /overflow-y: auto/);
  assert.match(styles, /overflow-x: auto/);
  assert.match(styles, /overflow: scroll/);
  assert.doesNotMatch(styles, /scrollbar-color:|::-webkit-scrollbar/);

  // The published stylesheet must carry the shared policy for both ordinary
  // native scroll containers and ScrollArea, including standalone consumers.
  const publishedStyles = await readFile(new URL("../dist/styles.css", import.meta.url), "utf8");
  assert.match(publishedStyles, /--alex-scrollbar-width/);
  assert.match(publishedStyles, /--alex-scrollbar-radius/);
  assert.match(publishedStyles, /--alex-color-scrollbar-thumb/);
  assert.match(publishedStyles, /--alex-color-scrollbar-thumb-hover/);
  assert.match(publishedStyles, /\[data-alex-scrollbar-visibility\]/);
  assert.match(publishedStyles, /scrollbar-width:\s*none/);
  assert.doesNotMatch(styles, /#[0-9a-f]{3,8}/i);
});
