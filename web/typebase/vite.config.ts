import { resolve } from "node:path";

import { defineConfig } from "vite";

export default defineConfig(({ command }) => ({
  base: command === "build" ? "/typebase/" : "/",
  plugins: [
    {
      name: "typebase-clean-reference-routes",
      configureServer(server) {
        server.middlewares.use((request, _response, next) => {
          const path = (request.url ?? "").split("?", 1)[0] ?? "";
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
