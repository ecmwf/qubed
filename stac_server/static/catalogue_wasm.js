/**
 * catalogue_wasm.js
 *
 * Loads the Rust/WebAssembly catalogue module and makes it available to
 * app.js via `window.__wasmCatalogue`.
 *
 * app.js's fetchCatalog() checks for window.__wasmCatalogue before doing a
 * network fetch; if it exists the WASM stac() method is called instead,
 * giving zero-latency, server-independent catalogue browsing.
 *
 * Graceful degradation: if anything fails the window.__wasmCatalogue is left
 * unset and app.js falls back to the server-side /api/v2/stac/ endpoint.
 */

import init, { WasmCatalogue } from "/static/wasm/qubed_wasm.js";

async function initWasm() {
  try {
    // 1. Initialise the WASM binary
    await init();

    const catalogue = new WasmCatalogue();

    // 2. Ask the server which data files to load
    const metaResp = await fetch("/api/v2/data_files");
    if (!metaResp.ok) {
      console.warn("[wasm] /api/v2/data_files returned", metaResp.status, "— falling back to server-side STAC");
      return;
    }
    const dataFiles = await metaResp.json(); // array of URL strings
    console.log("[wasm] Data files:", dataFiles);

    // 3. Load each data file into the catalogue
    let first = true;
    for (const url of dataFiles) {
      const resp = await fetch(url);
      if (!resp.ok) {
        console.warn(`[wasm] Could not fetch ${url} (${resp.status}) – skipping`);
        continue;
      }
      // The arena JSON endpoint returns a parsed JSON object; wasm expects a string
      const arenaJson = await resp.text();
      if (first) {
        catalogue.load(arenaJson);
        first = false;
      } else {
        catalogue.append(arenaJson);
      }
      console.log(`[wasm] Loaded ${url}`);
    }

    if (catalogue.is_empty()) {
      console.warn("[wasm] Catalogue empty after loading — falling back to server-side STAC");
      return;
    }

    // 4. Load MARS language metadata (descriptions, value labels)
    const langResp = await fetch("/api/v2/language");
    if (langResp.ok) {
      catalogue.set_language(await langResp.text());
      console.log("[wasm] Language metadata loaded");
    } else {
      console.warn("[wasm] /api/v2/language not available; descriptions will be empty");
    }

    // 5. Expose the catalogue to app.js and re-run the viewer
    window.__wasmCatalogue = catalogue;
    console.log("[wasm] WasmCatalogue ready — client-side catalogue browsing active");
    const badge = document.getElementById("wasm-status");
    if (badge) { badge.textContent = "🦀 WASM"; badge.style.background = "#d4edda"; badge.style.color = "#155724"; }

    // Re-run the viewer so it picks up the WASM catalogue for the current URL
    if (typeof window.initializeViewer === "function") {
      window.initializeViewer();
    }
  } catch (err) {
    console.error("[wasm] Initialisation failed:", err);
    // window.__wasmCatalogue remains unset → app.js uses server-side STAC
  }
}

initWasm();
