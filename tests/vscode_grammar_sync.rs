//! Keeps the VS Code extension in step with the language the lexer accepts.
//!
//! The extension in `editors/vscode/` restates the keyword set of the `.stage` language
//! twice — once as colouring rules, once as a documentation table — and neither copy can
//! notice that `src/loader/parser/lexer.rs` moved on. Two changes already queued in
//! `IDEAS.md` will move it: the `light` production, and the `substraction` → `subtraction`
//! rename. This test is what turns that silent drift into a red test suite.
//!
//! It reads the three files as plain text rather than parsing them, which costs nothing
//! and adds no dependency. In exchange, each file must keep the shape scanned here:
//!
//! - `lexer.rs`      — one `"keyword" => Token::KW…` arm per keyword;
//! - `*.tmLanguage.json` — every keyword rule written `\\b(?:a|b|c)\\b`, and that spelling
//!                     used for nothing else;
//! - `grammar.js`    — one quoted key per entry, opening its entry on its own line.

const LEXER: &str = include_str!("../src/loader/parser/lexer.rs");
const TEXTMATE: &str = include_str!("../editors/vscode/syntaxes/stage.tmLanguage.json");
const TABLE: &str = include_str!("../editors/vscode/src/grammar.js");

/// The content of the first pair of double quotes of a line.
fn first_quoted(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let end = line[start..].find('"')? + start;
    Some(line[start..end].to_string())
}

fn sorted(mut keywords: Vec<String>) -> Vec<String> {
    keywords.sort();
    keywords
}

/// The keywords the lexer maps to a token; anything else becomes `Token::Identifier`.
fn lexer_keywords() -> Vec<String> {
    sorted(
        LEXER
            .lines()
            .filter(|line| line.contains("=> Token::KW"))
            .filter_map(first_quoted)
            .collect(),
    )
}

/// The keywords the colouring rules enumerate, read out of their `\b(?:…)\b` alternations.
fn textmate_keywords() -> Vec<String> {
    let mut keywords = Vec::new();
    for alternation in TEXTMATE.split(r"\\b(?:").skip(1) {
        let group = alternation.split(')').next().unwrap_or_default();
        keywords.extend(group.split('|').map(str::to_string));
    }
    sorted(keywords)
}

/// The keywords the hover and completion table documents, read out of its quoted keys.
fn documented_keywords() -> Vec<String> {
    sorted(
        TABLE
            .lines()
            .filter(|line| line.trim_end().ends_with("\": {"))
            .filter_map(first_quoted)
            .collect(),
    )
}

#[test]
fn the_editor_colours_exactly_the_keywords_of_the_lexer() {
    let lexer = lexer_keywords();

    // A scan that quietly finds nothing would make this test pass for the wrong reason.
    assert!(
        lexer.len() > 30,
        "only {} keywords found in src/loader/parser/lexer.rs — the `\"kw\" => Token::KW…` shape this test scans for has changed",
        lexer.len()
    );

    assert_eq!(
        lexer,
        textmate_keywords(),
        "src/loader/parser/lexer.rs and editors/vscode/syntaxes/stage.tmLanguage.json disagree: a keyword changed on one side only, and .stage \
         files are now coloured wrong"
    );
}

#[test]
fn the_editor_documents_exactly_the_keywords_of_the_lexer() {
    assert_eq!(
        lexer_keywords(),
        documented_keywords(),
        "src/loader/parser/lexer.rs and editors/vscode/src/grammar.js disagree: a keyword is missing its hover entry, or documents something the \
         language no longer accepts"
    );
}
