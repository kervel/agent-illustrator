// Syntax highlighting for AIL: a stream tokenizer, not a full grammar — the
// renderer is the real parser, and it reports errors with line and column.
import { HighlightStyle, StreamLanguage, syntaxHighlighting } from "@codemirror/language";
import { tags as t } from "@lezer/highlight";

const KEYWORDS = new Set(
  (
    "row col grid stack group label template from export anchor place constrain keyframe show hide " +
    "transform disable enable import motion layout state set use as role contains midpoint inside " +
    "right-of left-of above below code"
  ).split(" "),
);
const SHAPES = new Set("rect circle ellipse polygon line icon text callout path vertex line_to arc_to curve_to close".split(" "));
const ATOMS = new Set(
  "left right top bottom center center_x center_y horizontal_center vertical_center up down clockwise cw counterclockwise ccw true false".split(" "),
);

interface State {
  inComment: boolean;
  inBrackets: number;
}

export const ail = StreamLanguage.define<State>({
  name: "ail",
  startState: () => ({ inComment: false, inBrackets: 0 }),
  token(stream, state) {
    if (state.inComment) {
      if (stream.skipTo("*/")) {
        stream.match("*/");
        state.inComment = false;
      } else stream.skipToEnd();
      return "blockComment";
    }
    if (stream.eatSpace()) return null;
    if (stream.match("//")) {
      stream.skipToEnd();
      return "lineComment";
    }
    if (stream.match("/*")) {
      state.inComment = true;
      return "blockComment";
    }
    if (stream.match(/^"(?:[^"\\]|\\.)*"?/)) return "string";
    if (stream.match(/^#[0-9a-fA-F]{3,8}\b/)) return "color";
    if (stream.match(/^-?\d+(\.\d+)?(%|px|ms|s|deg)?/)) return "number";
    if (stream.match(/^(<->|->|<-|--|>=|<=)/)) return "operator";
    const ch = stream.peek();
    if (ch === "[") state.inBrackets++;
    if (ch === "]") state.inBrackets = Math.max(0, state.inBrackets - 1);
    if (ch && "{}[]()".includes(ch)) {
      stream.next();
      return "bracket";
    }
    if (stream.match(/^[A-Za-z_][\w-]*/)) {
      const word = stream.current();
      // `[fill: accent-1]`: the key of a modifier.
      if (state.inBrackets > 0 && stream.match(/^\s*:/, false)) return "propertyName";
      if (KEYWORDS.has(word)) return "keyword";
      if (SHAPES.has(word)) return "typeName";
      if (ATOMS.has(word)) return "atom";
      return "variableName";
    }
    stream.next();
    return "punctuation";
  },
  languageData: { commentTokens: { line: "//", block: { open: "/*", close: "*/" } } },
});

/** Ink theme colours, tuned against the editor's navy background. */
export const ailHighlight = syntaxHighlighting(
  HighlightStyle.define([
    { tag: t.keyword, color: "#ffc247" },
    { tag: t.typeName, color: "#8fd3ff" },
    { tag: t.string, color: "#b5e48c" },
    { tag: [t.number, t.color], color: "#f6a5c0" },
    { tag: t.propertyName, color: "#c9b8ff" },
    { tag: t.atom, color: "#8fd3ff", fontStyle: "italic" },
    { tag: [t.lineComment, t.blockComment], color: "#6f81a3", fontStyle: "italic" },
    { tag: t.operator, color: "#ffc247" },
    { tag: [t.bracket, t.punctuation], color: "#8a9ab8" },
    { tag: t.variableName, color: "#e6ecf5" },
  ]),
);
