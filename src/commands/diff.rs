use anyhow::Result;
use std::fs;
use std::path::Path;
use lunar_interface::{ActualJson, build_structural_index, compare_routes, DiffResult};
use crate::adapter::run_adapter;

pub fn execute() -> Result<()> {
    let old_path = Path::new(".lunar").join(".interfaces-autogen.json");
    if !old_path.exists() { println!("No previous scan found. Run 'lunar scan' first."); return Ok(()); }
    let old_content = fs::read_to_string(&old_path)?;
    let old_actual: ActualJson = serde_json::from_str(&old_content)?;
    let old_routes = old_actual.exposed;
    let new_routes = run_adapter()?;
    let old_index = build_structural_index(&old_routes);
    let new_index = build_structural_index(&new_routes);
    let mut all_structs: Vec<String> = old_index.keys().cloned().collect();
    for k in new_index.keys() { if !all_structs.contains(k) { all_structs.push(k.clone()); } }
    all_structs.sort();
    let mut changes = Vec::new();
    for struct_id in &all_structs {
        let old_group = old_index.get(struct_id).cloned().unwrap_or_default();
        let new_group = new_index.get(struct_id).cloned().unwrap_or_default();
        if old_group.is_empty() { for nr in &new_group { changes.push(format!("  + {} {} (added)", nr.method, nr.display_path())); } continue; }
        if new_group.is_empty() { for or in &old_group { changes.push(format!("  - {} {} (removed)", or.method, or.display_path())); } continue; }
        let mut new_matched = vec![false; new_group.len()];
        let mut old_matched = vec![false; old_group.len()];
        for (oi, or) in old_group.iter().enumerate() {
            for (ni, nr) in new_group.iter().enumerate() {
                if new_matched[ni] { continue; }
                match compare_routes(or, nr) {
                    DiffResult::Unchanged => { old_matched[oi] = true; new_matched[ni] = true; break; }
                    DiffResult::ParamNamesChanged { old_names, new_names } => {
                        changes.push(format!("  ~ {} {} (param names: {:?} → {:?})", or.method, or.display_path(), old_names, new_names));
                        old_matched[oi] = true; new_matched[ni] = true; break;
                    }
                    DiffResult::MethodChanged { old_method, new_method } => {
                        changes.push(format!("  ~ {} {} → {} (method changed)", old_method, or.display_path(), new_method));
                        old_matched[oi] = true; new_matched[ni] = true; break;
                    }
                    _ => {}
                }
            }
        }
        for (oi, or) in old_group.iter().enumerate() { if !old_matched[oi] { changes.push(format!("  - {} {} (removed)", or.method, or.display_path())); } }
        for (ni, nr) in new_group.iter().enumerate() { if !new_matched[ni] { changes.push(format!("  + {} {} (added)", nr.method, nr.display_path())); } }
    }
    if changes.is_empty() { println!("No changes detected."); }
    else {
        println!("Changes detected:"); for line in &changes { println!("{}", line); }
        println!();
        println!("Hint: Copy the above output to your AI assistant to generate a contract patch.");
        println!("Then run `cat patch.yaml | lunar patch` to apply it.");
    }
    Ok(())
}
