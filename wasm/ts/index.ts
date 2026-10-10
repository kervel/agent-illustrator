/**
 * Agent Illustrator for the browser (and any other WebAssembly host).
 *
 * ```ts
 * import { init, Illustrator } from "agent-illustrator-wasm";
 * await init();
 * const ail = new Illustrator();
 * const { output } = ail.render("row { rect a  rect b }  a -> b");
 * ```
 *
 * Files are sans-io: the library never fetches anything. A document that
 * references other files (`import "deck.ail"`, `template "x" from "x.svg"`,
 * `code c [file: "main.py"]`) needs them supplied with `setFile`. Ask which
 * ones with `unresolved(source)`, or hand `renderResolved` a fetcher and let
 * it loop until everything it can get is in.
 */
import wasmInit, {
  Illustrator as RawIllustrator,
  docTopics as rawDocTopics,
  docs as rawDocs,
  playerJs as rawPlayerJs,
  takePanic,
  version as rawVersion,
  type InitInput,
} from "../pkg/agent_illustrator_wasm.js";

export type FileContents = string | Uint8Array | ArrayBuffer;

/** Returns a file's contents, or `null`/`undefined` when it does not exist. */
export type FileFetcher = (path: string) => Promise<FileContents | null | undefined>;

export interface RenderOptions {
  /** CSS layered over the default palette (the CLI's `--stylesheet-css`). */
  stylesheetCss?: string;
  /** `"none"` (default, static), `"js"` (embedded click-to-step player), `"css"` (self-playing, no JS). */
  animate?: "none" | "js" | "css";
  /** Skip the generated `.frame-X` rules; a host page drives the classes. */
  noFrameCss?: boolean;
  /** Render one keyframe (name or 0-based index) as a still. */
  frame?: string;
  /** With `frame`: sample its motion at `0.35s`, `350ms` or `50%`. */
  at?: string;
  /** Crop to what any frame shows, plus this margin. */
  cropToContent?: number;
  /** Also run the layout linter; findings join `diagnostics`. */
  lint?: boolean;
  /** Show element bounds and ids. */
  debug?: boolean;
  /** `"base64"` (default: inline images from supplied files) or `"verbatim"` (keep paths). */
  imageHref?: "base64" | "verbatim";
  /** Output the motion timeline as text instead of SVG. */
  timeline?: boolean;
  /** Output the storyboard (per click step: what is visible, what changes) as text. */
  states?: boolean;
}

export interface Diagnostic {
  severity: "error" | "warning";
  /** Stable kebab-case code: `syntax`, `undefined-identifier`, `missing-file`, ..., or a lint category (`overlap`, ...). */
  code: string;
  /** One line, without the location. */
  message: string;
  /** 1-based source range (end exclusive), when the diagnostic has one. */
  line?: number;
  column?: number;
  endLine?: number;
  endColumn?: number;
  /** What would fix it, when known (`did you mean: server?`). */
  hint?: string;
  /** Keyframes a lint finding occurs in; empty when frame-independent. */
  frames: string[];
  /** The elements a lint finding is about, as the message names them (dotted paths for parts). */
  elements: string[];
  /** The full report as the CLI prints it: location, source line, caret. */
  text: string;
}

export interface RenderResult {
  /** Rendered, and no diagnostic of `error` severity. */
  ok: boolean;
  /** The SVG (or text, for `timeline`/`states`); absent only when the render failed. Lint errors keep it. */
  output?: string;
  diagnostics: Diagnostic[];
  /** Files the render tried to read that were not supplied. */
  missingFiles: string[];
}

export interface Dependencies {
  /** Every file the document references, transitively through supplied files. */
  needed: string[];
  /** The part of `needed` not supplied yet. */
  missing: string[];
}

let ready: Promise<void> | undefined;

/** Load the WebAssembly module. Call once (repeat calls share the first). */
export function init(module?: InitInput | Promise<InitInput>): Promise<void> {
  ready ??= wasmInit(module === undefined ? undefined : { module_or_path: module }).then(() => undefined);
  return ready;
}

const encoder = new TextEncoder();

function toBytes(contents: FileContents): Uint8Array {
  if (typeof contents === "string") return encoder.encode(contents);
  return contents instanceof Uint8Array ? contents : new Uint8Array(contents);
}

/** A trap inside the module, as an error message (with the Rust panic text when there is one). */
function trapped(e: unknown): string {
  const panic = takePanic();
  return `internal error: ${panic ?? (e instanceof Error ? e.message : String(e))}`;
}

export class Illustrator {
  private raw: RawIllustrator;

  /** `basePath`: the directory the document lives in; its relative references resolve against it. */
  constructor(options: { basePath?: string; files?: Record<string, FileContents> } = {}) {
    this.raw = new RawIllustrator();
    if (options.basePath) this.raw.setBasePath(options.basePath);
    if (options.files) this.setFiles(options.files);
  }

  set basePath(base: string) {
    this.raw.setBasePath(base);
  }

  /** Supply a file the document references, by the path `unresolved` names. */
  setFile(path: string, contents: FileContents): this {
    this.raw.setFile(path, toBytes(contents));
    return this;
  }

  setFiles(files: Record<string, FileContents>): this {
    for (const [path, contents] of Object.entries(files)) this.setFile(path, contents);
    return this;
  }

  removeFile(path: string): boolean {
    return this.raw.removeFile(path);
  }

  clearFiles(): void {
    this.raw.clearFiles();
  }

  /** Paths of the supplied files. */
  files(): string[] {
    return this.raw.filePaths();
  }

  /** Every file `source` references, and which are still missing. */
  dependencies(source: string): Dependencies {
    return this.raw.dependencies(source) as Dependencies;
  }

  /** The files `source` references that have not been supplied yet. */
  unresolved(source: string): string[] {
    return this.dependencies(source).missing;
  }

  /** Render (and, with `lint: true`, lint) with the files supplied so far. Never throws for a bad document. */
  render(source: string, options: RenderOptions = {}): RenderResult {
    try {
      const raw = this.raw.render(source, options) as RenderResult;
      // serde writes absent fields as `undefined` already; keep the shape exact.
      return { ok: raw.ok, output: raw.output ?? undefined, diagnostics: raw.diagnostics, missingFiles: raw.missingFiles };
    } catch (e) {
      // Bad options throw a JsError; a trap is a RuntimeError.
      const message = e instanceof WebAssembly.RuntimeError ? trapped(e) : e instanceof Error ? e.message : String(e);
      const code = e instanceof WebAssembly.RuntimeError ? "internal" : "options";
      return { ok: false, diagnostics: [{ severity: "error", code, message, frames: [], elements: [], text: message }], missingFiles: [] };
    }
  }

  /**
   * Fetch every file `source` needs (repeatedly: a fetched file may import
   * more) and supply it. Returns the paths the fetcher could not provide.
   */
  async resolve(source: string, fetch: FileFetcher): Promise<string[]> {
    const unavailable = new Set<string>();
    for (;;) {
      const wanted = this.unresolved(source).filter((p) => !unavailable.has(p));
      if (wanted.length === 0) return [...unavailable];
      const got = await Promise.all(wanted.map((p) => fetch(p).catch(() => null)));
      wanted.forEach((path, i) => {
        const contents = got[i];
        if (contents == null) unavailable.add(path);
        else this.setFile(path, contents);
      });
    }
  }

  /**
   * `resolve`, then `render`; if the render still asked for files the static
   * scan missed, fetch those and render again.
   */
  async renderResolved(source: string, fetch: FileFetcher, options: RenderOptions = {}): Promise<RenderResult> {
    const unavailable = new Set(await this.resolve(source, fetch));
    for (;;) {
      const result = this.render(source, options);
      const wanted = result.missingFiles.filter((p) => !unavailable.has(p));
      if (wanted.length === 0) return result;
      for (const path of wanted) {
        const contents = await fetch(path).catch(() => null);
        if (contents == null) unavailable.add(path);
        else this.setFile(path, contents);
      }
      for (const p of await this.resolve(source, fetch)) unavailable.add(p);
    }
  }

  /** Keyframe names, in order (empty for a static diagram). */
  frameNames(source: string): string[] {
    try {
      return this.raw.frameNames(source);
    } catch {
      return [];
    }
  }

  /** Release the WebAssembly-side memory. */
  free(): void {
    this.raw.free();
  }
}

/** The library version (the release tag). */
export function version(): string {
  return rawVersion();
}

/** Reference docs: `"skill"`, `"brief"`, `"grammar"`, `"animation"`, `"styling"`, or a topic from `docTopics()`. */
export function docs(topic: string): string | undefined {
  return rawDocs(topic);
}

export function docTopics(): string[] {
  return rawDocTopics();
}

/** The standalone player script, for SVGs rendered with `noFrameCss`. */
export function playerJs(): string {
  return rawPlayerJs();
}
