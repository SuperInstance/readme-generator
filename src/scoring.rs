use crate::analysis::CrateInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreData {
    pub total: u64,
    pub max_total: u64,
    pub sections: Vec<SectionScore>,
    pub breakdown: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SectionScore {
    pub section: String,
    pub score: u64,
    pub max: u64,
    pub notes: String,
}

/// Score an existing README against what we'd expect from the crate info.
pub fn score_readme(readme: &str, info: &CrateInfo) -> ScoreData {
    let mut sections = Vec::new();
    let mut total = 0u64;
    let mut max_total = 0u64;

    // 1. Title present and matches crate name (0-10)
    let (s, m, n) = score_title(readme, info);
    total += s; max_total += m;
    sections.push(SectionScore { section: "Title".into(), score: s, max: m, notes: n });

    // 2. Description present (0-10)
    let (s, m, n) = score_description(readme, info);
    total += s; max_total += m;
    sections.push(SectionScore { section: "Description".into(), score: s, max: m, notes: n });

    // 3. Installation section (0-15)
    let (s, m, n) = score_installation(readme);
    total += s; max_total += m;
    sections.push(SectionScore { section: "Installation".into(), score: s, max: m, notes: n });

    // 4. Usage / Examples (0-15)
    let (s, m, n) = score_usage(readme);
    total += s; max_total += m;
    sections.push(SectionScore { section: "Usage".into(), score: s, max: m, notes: n });

    // 5. API Reference (0-15)
    let (s, m, n) = score_api_reference(readme, info);
    total += s; max_total += m;
    sections.push(SectionScore { section: "API Reference".into(), score: s, max: m, notes: n });

    // 6. Testing section (0-10)
    let (s, m, n) = score_testing(readme, info);
    total += s; max_total += m;
    sections.push(SectionScore { section: "Testing".into(), score: s, max: m, notes: n });

    // 7. License (0-10)
    let (s, m, n) = score_license(readme, info);
    total += s; max_total += m;
    sections.push(SectionScore { section: "License".into(), score: s, max: m, notes: n });

    // 8. Badges (0-5)
    let (s, m, n) = score_badges(readme);
    total += s; max_total += m;
    sections.push(SectionScore { section: "Badges".into(), score: s, max: m, notes: n });

    // 9. Dependencies section (0-5)
    let (s, m, n) = score_dependencies_section(readme, info);
    total += s; max_total += m;
    sections.push(SectionScore { section: "Dependencies".into(), score: s, max: m, notes: n });

    // 10. Formatting / quality (0-5)
    let (s, m, n) = score_formatting(readme);
    total += s; max_total += m;
    sections.push(SectionScore { section: "Formatting".into(), score: s, max: m, notes: n });

    ScoreData {
        total,
        max_total,
        sections,
        breakdown: format_score_breakdown_notes(readme),
    }
}

fn score_title(readme: &str, info: &CrateInfo) -> (u64, u64, String) {
    let max = 10u64;
    if readme.is_empty() {
        return (0, max, "No README content".into());
    }
    let first_line = readme.lines().next().unwrap_or("");
    if first_line.starts_with('#') && first_line.contains(&info.name) {
        (max, max, "Title present and matches crate name".into())
    } else if first_line.starts_with('#') {
        (7, max, "Title present but doesn't match crate name".into())
    } else {
        (2, max, "No markdown title (h1) found".into())
    }
}

fn score_description(readme: &str, info: &CrateInfo) -> (u64, u64, String) {
    let max = 10u64;
    if readme.is_empty() {
        return (0, max, "No content".into());
    }
    // Look for text after title
    let lines: Vec<&str> = readme.lines().collect();
    let desc_text = lines
        .iter()
        .skip_while(|l| l.starts_with('#') || l.is_empty())
        .take_while(|l| !l.starts_with('#') && !l.is_empty())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");

    if desc_text.is_empty() {
        return (2, max, "No description text found".into());
    }

    let mut score = 5u64;
    if let Some(expected) = &info.description {
        if desc_text.to_lowercase().contains(&expected.to_lowercase()) {
            score = max;
        }
    } else if desc_text.len() > 20 {
        score = 8;
    }
    (score, max, if score >= 8 { "Good description".into() } else { "Description could be improved".into() })
}

fn score_installation(readme: &str) -> (u64, u64, String) {
    let max = 15u64;
    let lower = readme.to_lowercase();
    if lower.contains("installation") || lower.contains("install") || lower.contains("cargo add") || lower.contains("cargo.toml") {
        let mut score = 8u64;
        if lower.contains("```toml") || lower.contains("```bash") {
            score = 12;
        }
        if lower.contains("dependencies") && (lower.contains("```toml") || lower.contains("```bash")) {
            score = max;
        }
        (score, max, "Installation section present".into())
    } else {
        (0, max, "No installation section found".into())
    }
}

fn score_usage(readme: &str) -> (u64, u64, String) {
    let max = 15u64;
    let lower = readme.to_lowercase();
    if lower.contains("usage") || lower.contains("example") {
        let mut score = 8u64;
        if lower.contains("```rust") {
            score = 12;
        }
        if lower.contains("```rust") && lower.matches("```").count() >= 4 {
            score = max;
        }
        (score, max, "Usage section with examples".into())
    } else if lower.contains("```rust") {
        (8, max, "Code examples present but no usage section header".into())
    } else {
        (0, max, "No usage or examples section".into())
    }
}

fn score_api_reference(readme: &str, info: &CrateInfo) -> (u64, u64, String) {
    let max = 15u64;
    if info.public_api.is_empty() {
        return (max, max, "No public API to document".into());
    }
    let lower = readme.to_lowercase();
    if lower.contains("api") || lower.contains("reference") {
        let mut score = 8u64;
        // Check if specific items are mentioned
        for s in &info.public_api.structs {
            if readme.contains(&s.name) { score += 1; break; }
        }
        for f in &info.public_api.functions {
            if readme.contains(&f.name) { score += 1; break; }
        }
        score = score.min(max);
        (score, max, "API reference section present".into())
    } else {
        // Check if public items are mentioned anywhere
        let mut mentioned = 0;
        for s in &info.public_api.structs {
            if readme.contains(&s.name) { mentioned += 1; }
        }
        for f in &info.public_api.functions {
            if readme.contains(&f.name) { mentioned += 1; }
        }
        if mentioned > 0 {
            (5, max, format!("{} API items mentioned but no API section", mentioned))
        } else {
            (0, max, "No API documentation found".into())
        }
    }
}

fn score_testing(readme: &str, info: &CrateInfo) -> (u64, u64, String) {
    let max = 10u64;
    if info.test_functions.is_empty() {
        return (max, max, "No tests in crate".into());
    }
    let lower = readme.to_lowercase();
    if lower.contains("test") {
        let mut score = 6u64;
        if lower.contains("cargo test") { score += 2; }
        if lower.contains("```bash") || lower.contains("```sh") { score += 2; }
        (score.min(max), max, "Testing section present".into())
    } else {
        (0, max, "No testing section found".into())
    }
}

fn score_license(readme: &str, info: &CrateInfo) -> (u64, u64, String) {
    let max = 10u64;
    let lower = readme.to_lowercase();
    if lower.contains("license") {
        let mut score = 6u64;
        if let Some(lic) = &info.license {
            if lower.contains(&lic.to_lowercase()) {
                score = max;
            }
        } else {
            score = 8;
        }
        (score, max, "License section present".into())
    } else {
        (0, max, "No license section".into())
    }
}

fn score_badges(readme: &str) -> (u64, u64, String) {
    let max = 5u64;
    let badge_count = readme.matches("shields.io").count();
    let score = badge_count.min(max as usize) as u64;
    (score, max, format!("{} badge(s) found", badge_count))
}

fn score_dependencies_section(readme: &str, info: &CrateInfo) -> (u64, u64, String) {
    let max = 5u64;
    if info.dependencies.is_empty() {
        return (max, max, "No dependencies".into());
    }
    let lower = readme.to_lowercase();
    if lower.contains("dependenc") {
        (max, max, "Dependencies section present".into())
    } else {
        (0, max, "No dependencies section".into())
    }
}

fn score_formatting(readme: &str) -> (u64, u64, String) {
    let max = 5u64;
    let mut score = 0u64;
    if !readme.is_empty() { score += 1; }
    if readme.lines().count() > 10 { score += 1; }
    if readme.contains("```") { score += 1; }
    if readme.matches("##").count() >= 3 { score += 1; }
    if !readme.trim_end().ends_with("#") { score += 1; }
    (score.min(max), max, "Formatting quality".into())
}

fn format_score_breakdown_notes(readme: &str) -> String {
    let mut notes = vec![];
    if readme.is_empty() {
        notes.push("Empty README".into());
    } else {
        let lines = readme.lines().count();
        let code_blocks = readme.matches("```").count() / 2;
        let headings = readme.matches("##").count();
        notes.push(format!("{} lines, {} code blocks, {} headings", lines, code_blocks, headings));
    }
    notes.join("; ")
}

pub fn format_score_report(data: &ScoreData) -> String {
    let mut report = String::new();
    let pct = if data.max_total > 0 {
        (data.total as f64 / data.max_total as f64 * 100.0) as u64
    } else {
        0
    };
    report.push_str(&format!("Score: {}/{} ({}%)\n\n", data.total, data.max_total, pct));
    report.push_str("Breakdown:\n");
    for section in &data.sections {
        let bar_len = 20;
        let filled = if section.max > 0 {
            (section.score as f64 / section.max as f64 * bar_len as f64).round() as usize
        } else {
            0
        };
        let bar: String = "█".repeat(filled) + &"░".repeat(bar_len - filled);
        report.push_str(&format!(
            "  {:20} [{}/{}] {} — {}\n",
            section.section,
            section.score,
            section.max,
            bar,
            section.notes
        ));
    }
    report
}
