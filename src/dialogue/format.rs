use std::collections::BTreeMap;

// Maybe move this to Display derive of the diff struct itself? idk ;)
pub(super) fn print_diff(diff: BTreeMap<String, Vec<(String, String)>>) -> String {
    let mut result = String::new();
    if diff.len() == 0 {
        result += "✅ Все правильно"
    } else {
        for (cat, diffs) in diff.iter() {
            result += format!("{}:\n", cat).as_str();
            for (ri, wr) in diffs {
                result += format!("✅ {}, ❌ {}\n", ri, wr).as_str();
            }
        }
    }

    result
}
