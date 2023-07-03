use anyhow::Result;
use once_cell::sync::Lazy;
use syntect::{
    html::{ClassStyle, ClassedHTMLGenerator},
    parsing::{SyntaxReference, SyntaxSet},
    util::LinesWithEndings,
};

static SYNTAXES: Lazy<SyntaxSet> = Lazy::new(|| {
    static SYNTAX_DATA: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/syntect.packdump"));

    syntect::dumps::from_uncompressed_data(SYNTAX_DATA).unwrap()
});

fn try_with_syntax(syntax: &SyntaxReference, code: &str) -> Result<String> {
    let mut html_generator = ClassedHTMLGenerator::new_with_class_style(
        syntax,
        &SYNTAXES,
        ClassStyle::SpacedPrefixed { prefix: "syntax-" },
    );

    for line in LinesWithEndings::from(code) {
        html_generator.parse_html_for_line_which_includes_newline(line)?;
    }

    Ok(html_generator.finalize())
}

fn select_syntax(name: Option<&str>, code: &str) -> &'static SyntaxReference {
    name.and_then(|name| {
        SYNTAXES.find_syntax_by_token(name).or_else(|| {
            name.rsplit_once('.')
                .and_then(|(_, ext)| SYNTAXES.find_syntax_by_token(ext))
        })
    })
    .or_else(|| SYNTAXES.find_syntax_by_first_line(code))
    .unwrap_or_else(|| SYNTAXES.find_syntax_plain_text())
}

pub(crate) fn try_with_lang(lang: Option<&str>, code: &str) -> Result<String> {
    try_with_syntax(select_syntax(lang, code), code)
}

#[cfg(test)]
mod tests {
    use super::{try_with_lang, select_syntax};

    #[test]
    fn custom_filetypes() {
        let toml = select_syntax(Some("toml"), "");

        assert_eq!(select_syntax(Some("Cargo.toml.orig"), "").name, toml.name);
        assert_eq!(select_syntax(Some("Cargo.lock"), "").name, toml.name);
    }

    #[test]
    fn dotfile_with_extension() {
        let toml = select_syntax(Some("toml"), "");

        assert_eq!(select_syntax(Some(".rustfmt.toml"), "").name, toml.name);
    }

    #[test]
    fn smoke() {
        assert_eq!(
            try_with_lang(Some("toml"), "[a]").unwrap(),
            r#"<span class="syntax-source syntax-toml"><span class="syntax-punctuation syntax-definition syntax-table syntax-begin syntax-toml">[</span><span class="syntax-meta syntax-tag syntax-table syntax-toml"><span class="syntax-entity syntax-name syntax-table syntax-toml">a</span></span><span class="syntax-punctuation syntax-definition syntax-table syntax-end syntax-toml">]</span></span>"#,
        );
    }
}
