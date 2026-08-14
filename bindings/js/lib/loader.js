"use strict";

// Carga de la librería nativa `clsb` (clsb.dll / libclsb.so / libclsb.dylib).
// Orden de búsqueda:
//   1. CLS_LIB_PATH (variable de entorno)
//   2. lib/ junto a este paquete (wheels/distribución la llevan)
//   3. PATH / rutas del sistema

const path = require("path");
const fs = require("fs");

function candidates() {
  const names =
    process.platform === "win32"
      ? ["clsb.dll"]
      : process.platform === "darwin"
        ? ["libclsb.dylib", "clsb.dylib"]
        : ["libclsb.so", "clsb.so"];
  const out = [];
  const env = process.env.CLS_LIB_PATH;
  if (env) {
    for (const n of names) out.push(path.join(env, n));
  }
  const pkgDir = path.join(__dirname, "..", "lib");
  for (const n of names) out.push(path.join(pkgDir, n));
  for (const n of names) out.push(n);
  return out;
}

function resolvePath() {
  let last = null;
  for (const cand of candidates()) {
    if (!fs.existsSync(cand)) continue;
    return cand;
  }
  throw new Error(
    "No se encontró la librería de CLS (clsb.dll/.so/.dylib). " +
      "Configura CLS_LIB_PATH o coloca el binario en bindings/js/lib/. " +
      `Buscado: ${candidates().join(", ")}${last ? ` (último error: ${last.message})` : ""}`
  );
}

function load() {
  const koffi = require("koffi");
  return koffi.load(resolvePath());
}

module.exports = { load, resolvePath, candidates };
