import path from "node:path";
import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";
import { createTokenAuthoringPlugin } from "./vite/token-authoring-plugin.mjs";
import { watchSourcePlugin } from "../../tooling/vite/watch-source.mjs";

const labDirectory = path.dirname(fileURLToPath(import.meta.url));
const designSystemDirectory = path.resolve(labDirectory, "../..");
const uiSourceDirectory = path.join(designSystemDirectory, "packages/ui/src");

export default defineConfig(({ command }) => ({
  plugins: [
    react(),
    watchSourcePlugin(uiSourceDirectory),
    createTokenAuthoringPlugin({ designSystemDirectory }),
  ],
  resolve: {
    dedupe: ["react", "react-dom"],
    alias:
      command === "serve"
        ? [
            {
              find: /^@alex\/ui\/flow-chat$/,
              replacement: path.join(uiSourceDirectory, "flow-chat.ts"),
            },
            {
              find: /^@alex\/ui\/registry$/,
              replacement: path.join(uiSourceDirectory, "registry.ts"),
            },
            {
              find: /^@alex\/ui\/styles\.css$/,
              replacement: path.join(uiSourceDirectory, "styles/layers.css"),
            },
            {
              find: /^@alex\/ui$/,
              replacement: path.join(uiSourceDirectory, "index.ts"),
            },
          ]
        : [],
  },
  optimizeDeps: {
    exclude: [
      "@alex/design-tokens",
      "@alex/theme-alex",
      "@alex/ui",
    ],
  },
  server: {
    fs: {
      allow: [designSystemDirectory],
    },
    host: "127.0.0.1",
    port: 4178,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    sourcemap: true,
  },
}));
