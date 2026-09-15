use std::fs;
use std::path::Path;

fn main() {
    let res_dir = Path::new("resources");
    if !res_dir.exists() {
        let _ = fs::create_dir_all(res_dir);
    }
    let rules_file = res_dir.join("rules.txt");
    if !rules_file.exists() {
        let _ = fs::write(
            rules_file,
            "# Caram Shield Baseline Rules\n||doubleclick.net^\n||google-analytics.com^\n",
        );
    }
    tauri_build::build();
}
