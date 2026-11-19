import { resolve } from "node:path";
import { existsSync, readdirSync } from "node:fs";

import { defineConfig } from "vite";

export default defineConfig(({ command, isPreview }) => ({
  // `vite preview` resolves this config with command "serve" but serves the
  // built directory, whose HTML references /typebase/assets/.
  base: command === "build" || isPreview ? "/typebase/" : "/",
  plugins: [
    {
      name: "typebase-clean-reference-routes",
      buildStart() {
        const root = resolve(__dirname, "public/data");
        if (existsSync(root) && readdirSync(root).some((name) => name.startsWith(".") && name.includes(".tmp-"))) {
          throw new Error("Unfinished projection directory under public/data; finish or remove that private work directory before building");
        }
      },
      configureServer(server) {
        server.middlewares.use((request, response, next) => {
          const path = (request.url ?? "").split("?", 1)[0] ?? "";
          if (path.split("/").some((part) => part.startsWith(".") && part.includes(".tmp-"))) {
            response.statusCode = 404;
            response.end("Not found");
            return;
          }
          if (/^\/Poset\/\d+\.-?\d+\.\d+\/?$/.test(path)) request.url = "/posets.html";
          else if (/^\/Steiner\/\d+\.-?\d+\.\d+\/?$/.test(path)) request.url = "/steiner.html";
          else if (
            path === "/Glossary" ||
            /^\/(Posets|Steiner)\/(Source|Completeness)\/?$/.test(path)
          ) {
            request.url = "/reference.html";
          }
          next();
        });
      },
    },
  ],
  build: {
    rollupOptions: {
      input: {
        home: resolve(__dirname, "index.html"),
        posets: resolve(__dirname, "posets.html"),
        steiner: resolve(__dirname, "steiner.html"),
        reference: resolve(__dirname, "reference.html"),
        notFound: resolve(__dirname, "404.html"),
      },
    },
  },
}));
