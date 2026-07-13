use std::sync::Arc;

use livesplit_auto_splitting::settings;

// .lss
pub fn parse_split_names(content: &str) -> Vec<String> {
    if content.is_empty() {
        return vec![];
    }
    let doc = match roxmltree::Document::parse(content) {
        Ok(d) => d,
        Err(_) => return vec![],
    };
    doc.descendants()
        .filter(|n| n.tag_name().name() == "Segment")
        .filter_map(|seg| {
            seg.children()
                .find(|n| n.tag_name().name() == "Name")
                .and_then(|n| n.text())
                .map(str::to_string)
        })
        .collect()
}

// Inner XML of `<AutoSplitterSettings>`
pub fn autosplitter_settings(content: &str) -> Option<&str> {
    let start = content.find("<AutoSplitterSettings>")? + "<AutoSplitterSettings>".len();
    let len = content[start..].find("</AutoSplitterSettings>")?;
    Some(&content[start..start + len])
}

pub fn build_settings_map(inner_xml: &str) -> settings::Map {
    let mut map = settings::Map::new();
    map.insert(
        Arc::from("autosplitter_settings"),
        settings::Value::String(Arc::from(inner_xml)),
    );
    map
}
