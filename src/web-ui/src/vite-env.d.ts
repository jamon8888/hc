/// <reference types="vite/client" />

declare module 'virtual:alex-canvas-runtime-bundle' {
  const bundle: {
    js: string;
    css: string;
  };
  export const alexCanvasRuntimeBundle: typeof bundle;
  export default bundle;
}
