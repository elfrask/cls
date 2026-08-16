"use strict";

// clsb - bindings de CLS para Node.js (embedding vía ABI C `clsb_v1_*`).
//
// API síncrona (los calls CLS son síncronos hoy): un Engine por thread; para
// concurrencia usa worker_threads (un engine por worker).

const loader = require("../lib/loader");

// ── kinds (códigos de la custom section clx:exports) ────────────────────────
const CLSB_INT = 0;
const CLSB_FLOAT = 1;
const CLSB_BOOL = 2;
const CLSB_CHAR = 3;
const CLSB_STRING = 4;
const CLSB_ARRAY = 5;
const CLSB_RECORD = 6;
const CLSB_NULL = 12;

// ── carga + firma de funciones ──────────────────────────────────────────────
let _lib = null;

function lib() {
  if (_lib) return _lib;
  const koffi = require("koffi");
  const raw = koffi.load(loader.resolvePath());
  const k = {};
  k._koffi = koffi;
  k._raw = raw;
  _lib = k;

  // Tipos (namespace global de koffi).
  koffi.opaque("clsb_engine");
  koffi.opaque("clsb_module");
  koffi.opaque("clsb_error");
  koffi.struct("clsb_config", {
    enable_fs: "int",
    enable_http: "int",
  });
  koffi.struct("clsb_value", {
    tag: "int32",
    bits: "int64",
    text: "void *",
    items: "clsb_value *",
    keys: "void *",
    vals: "clsb_value *",
    n: "size_t",
  });
  koffi.proto("clsb_output_cb", "void", ["void *", "str", "int"]);
  koffi.proto("clsb_resolver_cb", "size_t", ["void *", "str", "str", "void *", "size_t"]);
  koffi.proto("clsb_host_fn", "int", ["void *", "uint32", "clsb_value *", "size_t", "clsb_value *"]);

  const OutputCb = koffi.resolve("clsb_output_cb");
  const ResolverCb = koffi.resolve("clsb_resolver_cb");
  const HostFnCb = koffi.resolve("clsb_host_fn");
  k._types = {
    OutputCb: koffi.pointer(OutputCb),
    ResolverCb: koffi.pointer(ResolverCb),
    HostFnCb: koffi.pointer(HostFnCb),
  };

  const F = (name, ret, params) => (k[name] = raw.func(name, ret, params));
  const errRef = () => koffi.out(koffi.pointer("clsb_error", 2));
  F("clsb_engine_new", koffi.pointer("clsb_engine"), [koffi.pointer("clsb_config")]);
  F("clsb_engine_free", "void", [koffi.pointer("clsb_engine")]);
  F("clsb_compile_source", koffi.pointer("clsb_module"), [koffi.pointer("clsb_engine"), "str", "str", "str", errRef()]);
  F("clsb_compile_file", koffi.pointer("clsb_module"), [koffi.pointer("clsb_engine"), "str", errRef()]);
  F("clsb_module_free", "void", [koffi.pointer("clsb_module")]);
  F("clsb_run_main", "int64", [koffi.pointer("clsb_module"), "clsb_value *", "size_t", errRef()]);
  F("clsb_call", "int", [koffi.pointer("clsb_module"), "str", "clsb_value *", "size_t", koffi.out(koffi.pointer("clsb_value")), errRef()]);
  F("clsb_eval", "int", [koffi.pointer("clsb_engine"), "str", koffi.out(koffi.pointer("clsb_value")), errRef()]);
  F("clsb_set_output", "int", [koffi.pointer("clsb_engine"), koffi.pointer("clsb_output_cb"), "void *"]);
  F("clsb_set_resolver", "int", [koffi.pointer("clsb_engine"), koffi.pointer("clsb_resolver_cb"), "void *"]);
  F("clsb_register_host_function", "int", [koffi.pointer("clsb_engine"), "str", "str", koffi.pointer("clsb_host_fn"), "void *"]);  F("clsb_value_null", "clsb_value", []);
  F("clsb_value_int", "clsb_value", ["int64"]);
  F("clsb_value_float", "clsb_value", ["double"]);
  F("clsb_value_bool", "clsb_value", ["int"]);
  F("clsb_value_char", "clsb_value", ["uint32"]);
  F("clsb_value_string", "clsb_value", ["str"]);
  F("clsb_value_array", "clsb_value", ["size_t"]);
  F("clsb_value_record", "clsb_value", ["size_t"]);
  F("clsb_value_free", "void", [koffi.pointer("clsb_value")]);
  F("clsb_value_set_text", "void", [koffi.pointer("clsb_value"), "str"]);
  F("clsb_value_array_set", "void", [koffi.pointer("clsb_value"), "size_t", "clsb_value"]);
  F("clsb_value_record_set", "void", [koffi.pointer("clsb_value"), "size_t", "str", "clsb_value"]);
  F("clsb_error_free", "void", [koffi.pointer("clsb_error")]);
  F("clsb_error_trace", "str", [koffi.pointer("clsb_error")]);
  F("clsb_error_message", "str", [koffi.pointer("clsb_error")]);
  F("clsb_version", "str", []);

  // Utilidades koffi expuestas al resto del binding.
  k._koffi = koffi;
  k.register = (type, fn) => koffi.register(fn, type);
  k.alloc = (t, n) => koffi.alloc(t, n);
  k.asBuffer = (p, n) => koffi.view(p, n);
  k.null = koffi.null;
  k.pointer = (t) => koffi.pointer(t);
  return _lib;
}
// ── errores ─────────────────────────────────────────────────────────────────
class ClsError extends Error {
  constructor(message, trace) {
    super(trace || message);
    this.name = "ClsError";
    this.message = message;
    this.trace = trace || message;
  }
}

// koffi: clsb_error** -> pasar [null]; koffi escribe el puntero en [0].
function newErrPtr(k) {
  return [k._koffi.null];
}

function takeError(k, errPtr) {
  const p = errPtr[0];
  if (!p) return new ClsError("error desconocido", "error desconocido");
  const trace = k.clsb_error_trace(p) || "";
  const msg = k.clsb_error_message(p) || trace;
  k.clsb_error_free(p);
  errPtr[0] = null;
  return new ClsError(msg, trace);
}

// ── marshalling JS ↔ clsb_value ─────────────────────────────────────────────
// Los valores del host se construyen con los constructores C (copian con el
// runtime) y se liberan con clsb_value_free. Los hijos de array/record se
// escriben con los setters (el dueño es el contenedor).

function toValue(k, x) {
  if (x === null || x === undefined) return k.clsb_value_null();
  switch (typeof x) {
    case "boolean":
      return k.clsb_value_bool(x ? 1 : 0);
    case "number":
      if (Number.isInteger(x)) return k.clsb_value_int(x);
      return k.clsb_value_float(x);
    case "bigint":
      return k.clsb_value_int(Number(x));
    case "string":
      return k.clsb_value_string(x);
    case "object":
      if (Array.isArray(x)) {
        const v = k.clsb_value_array(x.length);
        const vp = [v];
        for (let i = 0; i < x.length; i++) {
          k.clsb_value_array_set(vp, i, toValue(k, x[i]));
        }
        return v;
      }
      {
        const keys = Object.keys(x);
        const v = k.clsb_value_record(keys.length);
        const vp = [v];
        for (let i = 0; i < keys.length; i++) {
          k.clsb_value_record_set(vp, i, keys[i], toValue(k, x[keys[i]]));
        }
        return v;
      }
    default:
      throw new TypeError(`tipo no soportado por el binding: ${typeof x}`);
  }
}

function readCString(k, ptr) {
  if (!ptr) return "";
  try {
    const ab = k.asBuffer(ptr, 4096);
    const bytes = Buffer.from(ab);
    const nul = bytes.indexOf(0);
    return bytes.subarray(0, nul === -1 ? bytes.length : nul).toString("utf8");
  } catch (err) {
    return "";
  }
}

function readValueNoFree(k, v) {
  switch (v.tag) {
    case CLSB_INT:
      return v.bits;
    case CLSB_FLOAT:
      return bitsToDouble(v.bits);
    case CLSB_BOOL:
      return v.bits !== 0;
    case CLSB_CHAR:
      return String.fromCodePoint(Number(v.bits));
    case CLSB_STRING:
      return readCString(k, v.text);
    case CLSB_ARRAY: {
      const arr = k._koffi.decode(v.items, k._koffi.array("clsb_value", v.n));
      return arr.map((item) => readValueNoFree(k, item));
    }
    case CLSB_RECORD: {
      const keys = k._koffi.decode(v.keys, k._koffi.array("char *", v.n));
      const vals = k._koffi.decode(v.vals, k._koffi.array("clsb_value", v.n));
      const out = {};
      for (let i = 0; i < v.n; i++) {
        out[keys[i]] = readValueNoFree(k, vals[i]);
      }
      return out;
    }
    default:
      return null;
  }
}

function fromValue(k, v) {
  // v es un clsb_value (struct copiado por koffi). Los punteros internos
  // (text/items/keys/vals) pertenecen al runtime C del módulo: NO se liberan
  // aquí (clsb_value_free espera un valor construido por el host; liberar este
  // copia con sus punteros sería doble-free/UB).
  return readValueNoFree(k, v);
}

function bitsToDouble(bits) {
  const buf = Buffer.alloc(8);
  buf.writeBigInt64LE(BigInt(bits));
  return buf.readDoubleLE();
}

// ── API ─────────────────────────────────────────────────────────────────────
class Engine {
  constructor(opts = {}) {
    const k = lib();
    this._k = k;
    // Sandbox por defecto: sin fs/http salvo que el embedder los pida.
    const cfg = {
      enable_fs: opts.fs ? 1 : 0,
      enable_http: opts.http ? 1 : 0,
    };
    this._h = k.clsb_engine_new(cfg);
    if (!this._h) throw new ClsError("no se pudo crear el engine", "clsb_engine_new falló");
    this._outputCb = null;
    this._resolverCb = null;
    this._hostCbs = [];
  }

  dispose() {
    if (this._h) {
      this._k.clsb_engine_free(this._h);
      this._h = null;
    }
  }

  get version() {
    return this._k.clsb_version() || "";
  }

  setOutput(cb) {
    const k = this._k;
    let buf = "";
    this._outputCb = k.register(k._types.OutputCb, (ud, text, isEnd) => {
      if (isEnd) {
        cb(buf);
        buf = "";
      } else if (text) {
        buf += text;
      }
    });
    k.clsb_set_output(this._h, this._outputCb, k.null);
  }

  setResolver(cb) {
    const k = this._k;
    this._resolverCb = k.register(k._types.ResolverCb, (ud, path, baseDir, bufPtr, bufLen) => {
      try {
        const src = cb(path, baseDir);
        if (src == null) return 0;
        const data = Buffer.from(src, "utf8");
        if (data.length > bufLen) return 0;
        Buffer.from(k.asBuffer(bufPtr, data.length)).set(data);
        return data.length;
      } catch {
        return 0;
      }
    });
    k.clsb_set_resolver(this._h, this._resolverCb, k.null);
  }

  registerHostFunction(name, sig, fn) {
    const k = this._k;
    const cb = k.register(k._types.HostFnCb, (ud, id, argsPtr, argsLen, outPtr) => {
      try {
        const arr = k._koffi.decode(argsPtr, k._koffi.array("clsb_value", argsLen));
        const args = arr.map((v) => readValueNoFree(k, v));
        const result = fn(id, args);
        k.clsb_value_free(outPtr);
        k._koffi.encode(outPtr, "clsb_value", toValue(k, result));
        return 0;
      } catch (e) {
        console.error(`clsb: host function ${id} falló:`, e);
        return 1;
      }
    });
    this._hostCbs.push(cb);
    const rc = k.clsb_register_host_function(this._h, name, sig, cb, k.null);
    if (rc !== 0) throw new ClsError(`no se pudo registrar '${name}'`, `sig inválida: ${sig}`);
  }

  compileSource(source, name = "module", baseDir = ".") {
    const k = this._k;
    const err = newErrPtr(k);
    const h = k.clsb_compile_source(this._h, source, name, baseDir, err);
    if (!h) throw takeError(k, err);
    return new Module(this, h);
  }

  compileFile(path) {
    const k = this._k;
    const err = newErrPtr(k);
    const h = k.clsb_compile_file(this._h, path, err);
    if (!h) throw takeError(k, err);
    return new Module(this, h);
  }

  eval(source) {
    const k = this._k;
    const err = newErrPtr(k);
    const out = k.clsb_value_null();
    const rc = k.clsb_eval(this._h, source, out, err);
    if (rc !== 0) throw takeError(k, err);
    return fromValue(k, out);
  }
}

class Module {
  constructor(engine, handle) {
    this._engine = engine;
    this._k = engine._k;
    this._h = handle;
  }

  dispose() {
    if (this._h) {
      this._k.clsb_module_free(this._h);
      this._h = null;
    }
  }

  runMain(args = []) {
    const k = this._k;
    const vals = args.map((a) => toValue(k, a));
    const arr = vals.length ? vals : [k.clsb_value_null()];
    const err = newErrPtr(k);
    const code = k.clsb_run_main(this._h, arr, vals.length, err);
    // Nota: los args no se liberan con clsb_value_free aquí. koffi copia los
    // structs a un buffer temporal y el text (CString del host) quedaría
    // doble-liberado si lo soltamos (el buffer temporal ya no existe).
    if (code === -1 && err[0]) throw takeError(k, err);
    return code;
  }

  call(name, ...args) {
    const k = this._k;
    const vals = args.map((a) => toValue(k, a));
    const arr = vals.length ? vals : [k.clsb_value_null()];
    const err = newErrPtr(k);
    const out = k.clsb_value_null();
    const rc = k.clsb_call(this._h, name, arr, vals.length, out, err);
    if (rc !== 0) throw takeError(k, err);
    return fromValue(k, out);
  }
}

module.exports = {
  Engine,
  Module,
  ClsError,
  version: () => lib().clsb_version() || "",
  CLSB_INT,
  CLSB_FLOAT,
  CLSB_BOOL,
  CLSB_CHAR,
  CLSB_STRING,
  CLSB_ARRAY,
  CLSB_RECORD,
  CLSB_NULL,
};
