// Diagnostics and results (spec/results.md, ADR-005).

/** One step of a path into a value: a map key (string) or an array index (number). */
export type PathSegment = string | number;

/** A problem found by an operation. */
export interface Diagnostic {
  /** A lower-case word with hyphens, such as `duplicate-key`. */
  readonly code: string;
  /** Text for a person; not part of the contract. */
  readonly message: string;
  /** The part of the value concerned; empty for the whole value. */
  readonly path: readonly PathSegment[];
}

/** The result of an operation that can find problems. A successful result may still carry diagnostics (losses). */
export type Result<T> =
  | { readonly ok: true; readonly value: T; readonly diagnostics: readonly Diagnostic[] }
  | { readonly ok: false; readonly diagnostics: readonly Diagnostic[] };

export function diagnostic(
  code: string,
  message: string,
  path: readonly PathSegment[] = [],
): Diagnostic {
  return { code, message, path };
}

export function ok<T>(value: T, diagnostics: readonly Diagnostic[] = []): Result<T> {
  return { ok: true, value, diagnostics };
}

export function fail<T = never>(...diagnostics: Diagnostic[]): Result<T> {
  return { ok: false, diagnostics };
}

/** The error thrown by {@link unwrap}. */
export class GramValueError extends Error {
  readonly diagnostics: readonly Diagnostic[];
  constructor(diagnostics: readonly Diagnostic[]) {
    super(diagnostics.map(formatDiagnostic).join("; "));
    this.name = "GramValueError";
    this.diagnostics = diagnostics;
  }
}

/** The value of a successful result; throws a {@link GramValueError} for a failed one. */
export function unwrap<T>(r: Result<T>): T {
  if (!r.ok) throw new GramValueError(r.diagnostics);
  return r.value;
}

/** A path as a JSON Pointer (RFC 6901): `/items/0/name`, empty for the whole value. */
export function pathToPointer(path: readonly PathSegment[]): string {
  return path
    .map((s) => "/" + String(s).replaceAll("~", "~0").replaceAll("/", "~1"))
    .join("");
}

export function formatDiagnostic(d: Diagnostic): string {
  const at = d.path.length > 0 ? ` (at ${pathToPointer(d.path)})` : "";
  return `${d.code}: ${d.message}${at}`;
}

/** Turn any loss into a failure, for a consumer that wants strictness (ADR-006 2.3). */
export function failOnLoss<T>(r: Result<T>): Result<T> {
  return r.ok && r.diagnostics.length > 0 ? { ok: false, diagnostics: r.diagnostics } : r;
}

/** Prefix every path of `diagnostics` with `segment`. */
export function under(
  segment: PathSegment,
  diagnostics: readonly Diagnostic[],
): Diagnostic[] {
  return diagnostics.map((d) => ({ ...d, path: [segment, ...d.path] }));
}
