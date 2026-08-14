// Tipos del binding `@cls-embed/node` (API síncrona).

export type ClsValue = number | bigint | boolean | string | ClsValue[] | { [k: string]: ClsValue } | null;

export declare class ClsError extends Error {
  message: string;
  trace: string;
  constructor(message: string, trace: string);
}

export declare class Module {
  private constructor();
  runMain(args?: string[]): number;
  call(name: string, ...args: ClsValue[]): ClsValue;
  dispose(): void;
}

export declare class Engine {
  constructor(opts?: EngineOptions);
  readonly version: string;
  setOutput(cb: (line: string) => void): void;
  setResolver(cb: (path: string, baseDir: string) => string | null | undefined): void;
  registerHostFunction(name: string, sig: string, fn: (id: number, args: ClsValue[]) => ClsValue): void;
  compileSource(source: string, name?: string, baseDir?: string): Module;
  compileFile(path: string): Module;
  eval(source: string): ClsValue;
  dispose(): void;
}

export interface EngineOptions {
  /** Expone el módulo `fs` (sandbox apagado para fs). Default: false. */
  fs?: boolean;
  /** Expone el módulo `http` (sandbox apagado para http). Default: false. */
  http?: boolean;
}

export declare function version(): string;

export declare const CLSB_INT: number;
export declare const CLSB_FLOAT: number;
export declare const CLSB_BOOL: number;
export declare const CLSB_CHAR: number;
export declare const CLSB_STRING: number;
export declare const CLSB_ARRAY: number;
export declare const CLSB_RECORD: number;
export declare const CLSB_NULL: number;
