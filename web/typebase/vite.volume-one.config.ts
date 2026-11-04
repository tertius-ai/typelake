import { copyFileSync, existsSync, renameSync } from "node:fs";
import { resolve } from "node:path";

import { defineConfig, type IndexHtmlTransformContext, type Plugin, type UserConfig } from "vite";

import baseConfig from "./vite.config";

const collectionRoute =
  /^\/(SimplicialComplex|SimplicialSet|FiniteGroupCohomology|EilenbergMacLane)(?:\/.*)?$/;

const masthead = `<header class="masthead typebase-masthead">
  <a class="wordmark" href="/typebase/" aria-label="TypeBase home">
    <span class="wordmark__name">TypeBase</span>
    <span class="wordmark__volume">Volume I</span>
  </a>
  <div class="proof-switcher" data-presentation-controls="site" aria-label="Presentation proof"></div>
  <nav class="typebase-nav" aria-label="Primary">
    <details class="collection-menu">
      <summary>Browse</summary>
      <div class="collection-menu__sheet">
        <p>Volume I collections</p>
        <a href="/typebase/posets.html" data-typebase-collection="Poset"><span>01</span><strong>Posets</strong></a>
        <a href="/typebase/steiner.html" data-typebase-collection="Steiner"><span>02</span><strong>Steiner categories</strong></a>
        <a href="/SimplicialComplex/" data-typebase-collection="SimplicialComplex"><span>03</span><strong>Simplicial complexes</strong></a>
        <a href="/SimplicialSet/" data-typebase-collection="SimplicialSet"><span>04</span><strong>Simplicial sets</strong></a>
        <a href="/FiniteGroupCohomology/" data-typebase-collection="FiniteGroupCohomology"><span>05</span><strong>Finite-group cohomology</strong></a>
        <a href="/EilenbergMacLane/" data-typebase-collection="EilenbergMacLane"><span>06</span><strong>Eilenberg–Mac Lane anima</strong></a>
      </div>
    </details>
    <a href="/Glossary">Glossary</a>
    <a href="https://tertius.cc/">Tertius</a>
  </nav>
</header>`;

function volumeOneRoutes(): Plugin {
  return {
    name: "typebase-volume-one-routes",
    configureServer(server) {
      server.middlewares.use((request, _response, next) => {
        const path = (request.url ?? "").split("?", 1)[0]?.replace(/\/+$/, "") || "/";
        const sourceShell = /^\/typebase\/(posets|steiner|reference)\.html$/.exec(path);
        if (sourceShell?.[1]) request.url = `/${sourceShell[1]}.html`;
        else if (collectionRoute.test(path)) request.url = "/collections.html";
        next();
      });
    },
    transformIndexHtml: {
      order: "post",
      handler(html: string, context?: IndexHtmlTransformContext) {
        if (context?.filename.endsWith("volume-one-404.html")) return html;
        return html.replace(
          /<header class="masthead(?: [^"]*)?" data-typebase-masthead><\/header>/,
          masthead,
        );
      },
    },
    writeBundle(options, bundle) {
      const routeLoader = Object.values(bundle).find(
        (entry) => entry.type === "asset" && entry.fileName.endsWith("volume-one-404.html"),
      );
      if (!routeLoader) throw new Error("the Volume I route loader was not emitted");
      const directory = options.dir ?? resolve(__dirname, "dist");
      renameSync(resolve(directory, routeLoader.fileName), resolve(directory, "404.html"));
      const posetShell = resolve(directory, "posets.html");
      if (!existsSync(posetShell)) copyFileSync(resolve(directory, "index.html"), posetShell);
    },
  };
}

export default defineConfig(async (environment) => {
  const resolved = (typeof baseConfig === "function"
    ? await baseConfig(environment)
    : await baseConfig) as UserConfig;
  const existing = resolved.build?.rollupOptions?.input;
  const input =
    typeof existing === "object" && existing !== null && !Array.isArray(existing)
      ? { ...existing }
      : {};
  input.collections = resolve(__dirname, "collections.html");
  input.notFound = resolve(__dirname, "volume-one-404.html");
  return {
    ...resolved,
    plugins: [...(resolved.plugins ?? []), volumeOneRoutes()],
    build: {
      ...resolved.build,
      rollupOptions: {
        ...resolved.build?.rollupOptions,
        input,
      },
    },
  };
});
