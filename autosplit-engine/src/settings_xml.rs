//! Reader for the `.lss` `<AutoSplitterSettings>` XML

use alloc::{format, string::String};

pub fn xml_unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        rest = &rest[pos..];
        let (rep, len) = if rest.starts_with("&amp;") {
            ("&", 5)
        } else if rest.starts_with("&lt;") {
            ("<", 4)
        } else if rest.starts_with("&gt;") {
            (">", 4)
        } else if rest.starts_with("&quot;") {
            ("\"", 6)
        } else if rest.starts_with("&apos;") {
            ("'", 6)
        } else {
            ("&", 1)
        };
        out.push_str(rep);
        rest = &rest[len..];
    }
    out.push_str(rest);
    out
}

// Iterate text of every "<tag>..</tag>"
pub fn xml_texts<'x>(xml: &'x str, tag: &str) -> impl Iterator<Item = &'x str> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut rest = xml;
    core::iter::from_fn(move || {
        let start = rest.find(open.as_str())? + open.len();
        let len = rest[start..].find(close.as_str())?;
        let text = &rest[start..start + len];
        rest = &rest[start + len + close.len()..];
        Some(text)
    })
}
