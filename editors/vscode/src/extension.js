// Hover and completion for the `.stage` scene language.
//
// Everything shown to the reader comes from `grammar.js`; this file only decides *what*
// to show and *when*. There is no parser here on purpose: the real one lives in
// `src/loader/parser.rs` and panics on the first mistake, which is no basis for an
// editor. What follows counts tokens instead, and says so when it stops knowing.

const vscode = require("vscode");
const { PRODUCTIONS, group } = require("./grammar");

// The three operators that may stand between `csg` and its brace.
const CSG_OPS = ["union", "intersection", "substraction"];

// Splits a `.stage` source into the same tokens the lexer would produce, minus the
// position bookkeeping. Comments are dropped, and anything the lexer refuses simply
// does not match — a stray `,` leaves no token, exactly as it leaves none in Rust.
function tokenize(text) {
  const pattern = /#[^\n]*|"[^"]*"|[+-]?[0-9]+(?:\.[0-9]*)?|[A-Za-z][A-Za-z0-9_]*|[{}]/g;
  const tokens = [];
  let match;

  while ((match = pattern.exec(text)) !== null) {
    const lexeme = match[0];
    if (lexeme.startsWith("#")) {
      continue;
    }
    else if (lexeme.startsWith('"')) {
      tokens.push({ kind: "string", value: lexeme });
    }
    else if (lexeme === "{" || lexeme === "}") {
      tokens.push({ kind: lexeme, value: lexeme });
    }
    else if (/^[+-]?[0-9]/.test(lexeme)) {
      tokens.push({ kind: "number", value: lexeme });
    }
    else {
      tokens.push({ kind: "word", value: lexeme });
    }
  }

  return tokens;
}

// The stack of open blocks, each named after the keyword that opened it. Only three
// constructs take braces, so the keyword sitting just before `{` identifies the block.
function openBlocks(tokens) {
  const stack = [];

  for (let i = 0; i < tokens.length; i += 1) {
    if (tokens[i].kind === "{") {
      const opener = i > 0 ? tokens[i - 1].value : "";
      if (opener === "transform") {
        stack.push("transform");
      }
      else if (CSG_OPS.includes(opener)) {
        stack.push("csg");
      }
      else if (opener === "compound") {
        stack.push("compound");
      }
      else {
        stack.push("unknown");
      }
    }
    else if (tokens[i].kind === "}") {
      stack.pop();
    }
  }

  return stack;
}

// What the grammar expects at the cursor.
//
// Returns a list of keywords, `[]` when a number or a string is expected — saying
// nothing is more honest than proposing a keyword the parser would refuse — or `null`
// when the automaton has lost the thread, in which case the caller offers everything.
function expectation(tokens) {
  if (tokens.length === 0) {
    return ["camera"];
  }

  const stack = openBlocks(tokens);
  const block = stack.length > 0 ? stack[stack.length - 1] : "root";

  // Walk back over the literals typed since the last keyword: their count is what tells
  // a finished production from one still waiting for a number.
  let anchor = tokens.length - 1;
  let literals = 0;
  while (anchor >= 0 && (tokens[anchor].kind === "number" || tokens[anchor].kind === "string")) {
    literals += 1;
    anchor -= 1;
  }
  if (anchor < 0) {
    return null;
  }

  const token = tokens[anchor];

  if (token.kind === "{") {
    if (block === "transform") {
      return group("transform_step");
    }
    else if (block === "csg") {
      return ["elem"];
    }
    else if (block === "compound") {
      return ["object"];
    }
    return null;
  }

  if (token.kind === "}") {
    if (block === "csg") {
      return ["elem"];
    }
    else if (block === "compound") {
      return ["object"];
    }
    else if (block === "root") {
      return ["object"];
    }
    return null;
  }

  const production = PRODUCTIONS[token.value];
  const afterShape = () => (block === "csg" ? ["transform"] : group("material"));

  switch (token.value) {
    case "camera":
      return group("camera_type");
    case "pin_hole":
    case "thin_lens":
      return ["pos"];
    case "pos":
      return literals >= 3 ? ["look"] : [];
    case "look":
      return literals >= 3 ? ["up"] : [];
    case "up":
      if (literals < 3) {
        return [];
      }
      return tokens.some((t) => t.value === "thin_lens") ? ["radius"] : ["scene"];
    case "radius":
      return literals >= 1 ? ["focal_length"] : [];
    case "focal_length":
      return literals >= 1 ? ["scene"] : [];

    case "scene":
      return ["object"];
    case "object":
      return group("object_kind");
    case "simple":
    case "elem":
      return group("shape");
    case "transformed":
      return ["object"];

    case "sphere":
    case "rectangle":
    case "plane":
    case "cylinder":
    case "aabox":
      return literals >= production.arity ? afterShape() : [];
    case "mesh":
      return ["file"];
    case "file":
      return literals >= 1 ? ["reverse"].concat(afterShape()) : [];
    case "reverse":
      return afterShape();
    case "csg":
      return group("csg_op");

    case "lambertian":
    case "diffuse_light":
      return group("texture");
    case "metal":
    case "dielectric":
      return literals >= 1 ? group("texture") : [];

    case "color":
    case "checkerboard":
      return literals >= production.arity ? ["transform", "object"] : [];

    case "translate":
    case "rotate_x":
    case "rotate_y":
    case "rotate_z":
      return literals >= production.arity ? group("transform_step") : [];

    // `compound`, `transform` and the CSG operators all wait for a brace, which the
    // reader types rather than picks from a list.
    case "compound":
    case "transform":
    case "union":
    case "intersection":
    case "substraction":
      return [];

    default:
      return null;
  }
}

// Renders one production as hover markdown: signature, prose, parameters, example, and
// a link to the code that decides the semantics.
function documentation(keyword) {
  const production = PRODUCTIONS[keyword];
  const markdown = new vscode.MarkdownString();

  markdown.appendCodeblock(production.signature, "stage");
  markdown.appendMarkdown(production.summary + "\n\n");

  for (const parameter of production.params) {
    markdown.appendMarkdown(`- \`${parameter.name}\` — *${parameter.type}* — ${parameter.doc}\n`);
  }
  if (production.params.length > 0) {
    markdown.appendMarkdown("\n");
  }

  markdown.appendCodeblock(production.example, "stage");
  markdown.appendMarkdown(sourceLink(production.source));

  return markdown;
}

// Turns `src/shapes/rectangle.rs:16` into a link into the open workspace, or leaves it
// as plain text when the extension runs outside the pbrt folder.
function sourceLink(source) {
  const folders = vscode.workspace.workspaceFolders;
  const [path, line] = source.split(":");

  if (!folders || folders.length === 0) {
    return `\n\`${source}\``;
  }

  const uri = vscode.Uri.joinPath(folders[0].uri, path).with({ fragment: line ? `L${line}` : "" });
  return `\n[${source}](${uri})`;
}

const COMPLETION_KIND = {
  structure: vscode.CompletionItemKind.Keyword,
  camera_type: vscode.CompletionItemKind.Class,
  object_kind: vscode.CompletionItemKind.Struct,
  shape: vscode.CompletionItemKind.Class,
  csg_op: vscode.CompletionItemKind.Operator,
  material: vscode.CompletionItemKind.Color,
  texture: vscode.CompletionItemKind.Color,
  transform_step: vscode.CompletionItemKind.Function,
  label: vscode.CompletionItemKind.Property,
  flag: vscode.CompletionItemKind.Constant,
};

function completionItem(keyword, rank) {
  const production = PRODUCTIONS[keyword];
  const item = new vscode.CompletionItem(keyword, COMPLETION_KIND[production.group]);

  item.detail = production.signature;
  item.documentation = documentation(keyword);
  // The proposed order is the order the language reads in, not the alphabet.
  item.sortText = String(rank).padStart(2, "0");
  if (production.snippet) {
    item.insertText = new vscode.SnippetString(production.snippet);
  }

  return item;
}

const hoverProvider = {
  provideHover(document, position) {
    const wordRange = document.getWordRangeAtPosition(position);
    if (wordRange) {
      const word = document.getText(wordRange);
      if (PRODUCTIONS[word]) {
        return new vscode.Hover(documentation(word), wordRange);
      }
    }

    // Angles are written in radians, which no one reads at a glance. On a number that
    // follows a rotation, show what it actually turns.
    const numberRange = document.getWordRangeAtPosition(position, /[+-]?[0-9]+(?:\.[0-9]*)?/);
    if (!numberRange) {
      return undefined;
    }

    const preceding = tokenize(document.getText(new vscode.Range(new vscode.Position(0, 0), numberRange.start)));
    const last = preceding[preceding.length - 1];
    if (!last || !["rotate_x", "rotate_y", "rotate_z"].includes(last.value)) {
      return undefined;
    }

    const radians = Number(document.getText(numberRange));
    const degrees = (radians * 180) / Math.PI;
    const markdown = new vscode.MarkdownString(`\`${radians}\` rad ≈ **${degrees.toFixed(2)}°**`);

    return new vscode.Hover(markdown, numberRange);
  },
};

const completionProvider = {
  provideCompletionItems(document, position) {
    const upToCursor = document.getText(new vscode.Range(new vscode.Position(0, 0), position));
    const tokens = tokenize(upToCursor);

    // A word being typed is not yet a token to reason from: drop it, the editor filters
    // the proposals against it anyway.
    const wordRange = document.getWordRangeAtPosition(position);
    if (wordRange && !wordRange.isEmpty && tokens.length > 0 && tokens[tokens.length - 1].kind === "word") {
      tokens.pop();
    }

    const expected = expectation(tokens);
    const keywords = expected === null ? Object.keys(PRODUCTIONS) : expected;

    return keywords.map((keyword, rank) => completionItem(keyword, rank));
  },
};

function activate(context) {
  context.subscriptions.push(
    vscode.languages.registerHoverProvider("stage", hoverProvider),
    vscode.languages.registerCompletionItemProvider("stage", completionProvider),
  );
}

function deactivate() {}

module.exports = { activate, deactivate };
