import { dlopen, FFIType, ptr, CString } from "bun:ffi";
import { findBinary } from "./loader";

let _lib: ReturnType<typeof loadLib> | null = null;

function loadLib() {
  const path = findBinary();
  return dlopen(path, {
    diceng_parse: {
      args: [FFIType.cstring],
      returns: FFIType.ptr,
    },
    diceng_roll: {
      args: [FFIType.cstring, FFIType.i64],
      returns: FFIType.ptr,
    },
    diceng_roll_dice: {
      args: [FFIType.cstring, FFIType.i64],
      returns: FFIType.ptr,
    },
    diceng_compute_distribution: {
      args: [FFIType.cstring, FFIType.u64],
      returns: FFIType.ptr,
    },
    diceng_free: {
      args: [FFIType.ptr],
      returns: FFIType.void,
    },
  });
}

function getLib() {
  if (!_lib) {
    _lib = loadLib();
  }
  return _lib;
}

function callJson(
  fn: (ptr: number, ...args: unknown[]) => number,
  input: string,
  ...args: unknown[]
): unknown {
  const buf = Buffer.from(input + "\0", "utf8");
  const resultPtr = fn(ptr(buf), ...args);
  if (!resultPtr) {
    throw new Error("diceng FFI returned null pointer");
  }
  const cstr = new CString(resultPtr);
  getLib().symbols.diceng_free(resultPtr);
  return JSON.parse(cstr.toString());
}

export function ffi_parse(input: string): unknown {
  return callJson(getLib().symbols.diceng_parse, input);
}

export function ffi_roll(exprJson: string, seed: number): unknown {
  return callJson(getLib().symbols.diceng_roll, exprJson, seed);
}

export function ffi_roll_dice(input: string, seed: number): unknown {
  return callJson(getLib().symbols.diceng_roll_dice, input, seed);
}

export function ffi_compute_distribution(
  exprJson: string,
  trials: number
): unknown {
  return callJson(
    getLib().symbols.diceng_compute_distribution,
    exprJson,
    trials
  );
}
