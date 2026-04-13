import { defineConfig } from "@pandacss/dev";
import pandaPreset from "@pandacss/preset-panda";
import { createPreset } from "@park-ui/panda-preset";

export default defineConfig({
  preflight: true,
  include: ["./src/**/*.{js,jsx,ts,tsx}", "./pages/**/*.{js,jsx,ts,tsx}"],
  exclude: [],
  presets: [
    pandaPreset,
    createPreset({
      accentColor: "blue",
      grayColor: "slate",
      borderRadius: "md",
    }),
  ],
  theme: { extend: {} },
  outdir: "src/styled-system",
  jsxFramework: "solid",
});
