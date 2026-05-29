use similar::{ChangeTag, TextDiff};

/// Generate a unified diff between two text strings.
pub fn unified_diff(old_text: &str, new_text: &str, old_label: &str, new_label: &str) -> String {
    let diff = TextDiff::from_lines(old_text, new_text);
    let mut output = format!("--- {old_label}\n+++ {new_label}\n");

    for group in diff.grouped_ops(3) {
        for op in &group {
            for change in diff.iter_changes(op) {
                let sign = match change.tag() {
                    ChangeTag::Delete => "-",
                    ChangeTag::Insert => "+",
                    ChangeTag::Equal => " ",
                };
                output.push_str(&format!("{sign}{}", change.value()));
            }
        }
    }

    output
}
