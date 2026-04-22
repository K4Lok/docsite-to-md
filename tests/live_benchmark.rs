use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use docsite_to_md::{
    BrowserOptions, CrawlOptions, ExportOptions, ExportResult, Framework, detect_site, export_site,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Deserialize)]
struct LiveTarget {
    name: String,
    framework: Framework,
    entry_url: String,
    scope_prefix: Option<String>,
    max_pages: usize,
    representative_urls: Vec<String>,
    notes: String,
}

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    generated_at_epoch: u64,
    targets: Vec<TargetReport>,
}

#[derive(Debug, Serialize)]
struct TargetReport {
    name: String,
    framework: Framework,
    entry_url: String,
    scope_prefix: Option<String>,
    smoke_passed: bool,
    exported_pages: usize,
    representative_pages_found: Vec<String>,
    representative_pages_missing: Vec<String>,
    target_grade: QualityGrade,
    page_reports: Vec<PageReport>,
    notes: String,
}

#[derive(Debug, Serialize)]
struct PageReport {
    url: String,
    output_file: String,
    used_browser_fallback: bool,
    grade: QualityGrade,
    findings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
enum QualityGrade {
    Pass,
    Warn,
    Fail,
}

macro_rules! live_smoke_test {
    ($test_name:ident, $target_name:literal) => {
        #[tokio::test]
        #[ignore = "live smoke benchmark"]
        async fn $test_name() {
            let target = find_target($target_name);
            let result = run_target_smoke(&target)
                .await
                .unwrap_or_else(|error| panic!("{} smoke failed: {error}", target.name));
            assert!(
                result.representative_found,
                "{} representative export missing",
                target.name
            );
        }
    };
}

live_smoke_test!(live_smoke_mycactus_gitbook, "mycactus-gitbook");
live_smoke_test!(live_smoke_jest, "jest");
live_smoke_test!(live_smoke_redux, "redux");
live_smoke_test!(live_smoke_react_native, "react-native");
live_smoke_test!(live_smoke_mkdocs_material, "mkdocs-material");
live_smoke_test!(live_smoke_vitepress, "vitepress");
live_smoke_test!(live_smoke_vitest, "vitest");
live_smoke_test!(live_smoke_pinia, "pinia");
live_smoke_test!(live_smoke_nextra, "nextra");

#[tokio::test]
#[ignore = "live quality benchmark"]
async fn live_benchmark_report() {
    let mut reports = Vec::new();

    for target in load_targets() {
        match run_target_report(&target).await {
            Ok(report) => reports.push(report),
            Err(error) => reports.push(TargetReport {
                name: target.name.clone(),
                framework: target.framework.clone(),
                entry_url: target.entry_url.clone(),
                scope_prefix: target.scope_prefix.clone(),
                smoke_passed: false,
                exported_pages: 0,
                representative_pages_found: Vec::new(),
                representative_pages_missing: target.representative_urls.clone(),
                target_grade: QualityGrade::Fail,
                page_reports: vec![PageReport {
                    url: target.entry_url.clone(),
                    output_file: String::new(),
                    used_browser_fallback: false,
                    grade: QualityGrade::Fail,
                    findings: vec![error],
                }],
                notes: target.notes.clone(),
            }),
        }
    }

    let summary = BenchmarkReport {
        generated_at_epoch: now_epoch(),
        targets: reports,
    };
    let summary_json = serde_json::to_string_pretty(&summary).expect("summary should serialize");
    let report_dir = benchmark_output_dir("summary");
    fs::create_dir_all(&report_dir).expect("report directory should create");
    let report_path = report_dir.join("live-benchmark-summary.json");
    fs::write(&report_path, &summary_json).expect("summary should write");

    println!("Live benchmark summary: {}", report_path.display());
    for target in &summary.targets {
        println!(
            "[{:?}] {} smoke={} exported={} found={}/{}",
            target.target_grade,
            target.name,
            target.smoke_passed,
            target.exported_pages,
            target.representative_pages_found.len(),
            target.representative_pages_found.len() + target.representative_pages_missing.len()
        );
        for page in &target.page_reports {
            println!(
                "  - [{:?}] {} fallback={} {}",
                page.grade, page.url, page.used_browser_fallback, page.output_file
            );
            for finding in &page.findings {
                println!("    * {finding}");
            }
        }
    }
    println!("{summary_json}");
}

#[derive(Debug)]
struct SmokeArtifacts {
    representative_found: bool,
}

async fn run_target_smoke(target: &LiveTarget) -> Result<SmokeArtifacts, String> {
    let profile = detect_site(&target.entry_url)
        .await
        .map_err(|error| format!("detect failed: {error}"))?;
    if profile.framework != target.framework {
        return Err(format!(
            "expected framework {:?}, got {:?}",
            target.framework, profile.framework
        ));
    }

    let export = export_target(target)
        .await
        .map_err(|error| format!("export failed: {error}"))?;
    let representative_found = target
        .representative_urls
        .iter()
        .any(|url| has_exported_page_for_url(&export, url));
    if !representative_found {
        return Err("no representative markdown file was exported".to_string());
    }

    Ok(SmokeArtifacts {
        representative_found,
    })
}

async fn run_target_report(target: &LiveTarget) -> Result<TargetReport, String> {
    let profile = detect_site(&target.entry_url)
        .await
        .map_err(|error| format!("detect failed: {error}"))?;
    let expected_framework = profile.framework == target.framework;
    let export = export_target(target)
        .await
        .map_err(|error| format!("export failed: {error}"))?;

    let representative_pages_found = target
        .representative_urls
        .iter()
        .filter(|url| has_exported_page_for_url(&export, url))
        .cloned()
        .collect::<Vec<_>>();
    let representative_pages_missing = target
        .representative_urls
        .iter()
        .filter(|url| !has_exported_page_for_url(&export, url))
        .cloned()
        .collect::<Vec<_>>();
    let smoke_passed = expected_framework && !representative_pages_found.is_empty();

    let mut page_reports = Vec::new();
    for url in &target.representative_urls {
        if let Some(page) = export
            .pages
            .iter()
            .find(|page| canonicalize_live_url(&page.page.url) == canonicalize_live_url(url))
        {
            let markdown_path = export.output_dir.join(&page.output_file);
            if markdown_path.exists() {
                let markdown = fs::read_to_string(&markdown_path).map_err(|error| {
                    format!("unable to read {}: {error}", markdown_path.display())
                })?;
                let (grade, findings) = evaluate_markdown_quality(&markdown);
                page_reports.push(PageReport {
                    url: url.clone(),
                    output_file: page.output_file.clone(),
                    used_browser_fallback: page.used_browser_fallback,
                    grade,
                    findings,
                });
            }
        }
    }

    if page_reports.is_empty() {
        page_reports.push(PageReport {
            url: target.entry_url.clone(),
            output_file: relative_output_path_for_url(&target.entry_url),
            used_browser_fallback: false,
            grade: QualityGrade::Fail,
            findings: vec![
                "no representative pages were available for quality scoring".to_string(),
            ],
        });
    }

    let target_grade = page_reports
        .iter()
        .map(|page| page.grade)
        .max()
        .unwrap_or(QualityGrade::Warn);

    Ok(TargetReport {
        name: target.name.clone(),
        framework: target.framework.clone(),
        entry_url: target.entry_url.clone(),
        scope_prefix: target.scope_prefix.clone(),
        smoke_passed,
        exported_pages: export.pages.len(),
        representative_pages_found,
        representative_pages_missing,
        target_grade: if !smoke_passed {
            QualityGrade::Fail
        } else {
            target_grade
        },
        page_reports,
        notes: target.notes.clone(),
    })
}

async fn export_target(target: &LiveTarget) -> docsite_to_md::Result<ExportResult> {
    let output = benchmark_output_dir(&target.name);
    fs::create_dir_all(&output).expect("benchmark output directory should create");
    export_site(
        &target.entry_url,
        ExportOptions {
            output_dir: output,
            crawl: CrawlOptions {
                scope_prefix: target.scope_prefix.clone(),
                excludes: Vec::new(),
                retry_attempts: 2,
                rate_limit_ms: 0,
                max_concurrency: 4,
                max_pages: Some(target.max_pages),
            },
            resume: false,
            bundle_output: None,
            browser: BrowserOptions::default(),
        },
    )
    .await
}

fn evaluate_markdown_quality(markdown: &str) -> (QualityGrade, Vec<String>) {
    let mut findings = Vec::new();
    let mut grade = QualityGrade::Pass;

    let heading_present = markdown.starts_with("# ")
        || markdown.contains("\n# ")
        || markdown
            .lines()
            .collect::<Vec<_>>()
            .windows(2)
            .any(|window| window[0].trim().len() >= 2 && is_setext_underline(window[1]));
    if !heading_present {
        grade = QualityGrade::Fail;
        findings.push("missing clean top-level heading".to_string());
    }

    if markdown.trim().len() < 80 {
        grade = QualityGrade::Fail;
        findings.push("markdown is unexpectedly short".to_string());
    }

    let code_fence_count = markdown.matches("```").count();
    if code_fence_count % 2 != 0 {
        grade = grade.max(QualityGrade::Warn);
        findings.push("unbalanced code fences".to_string());
    }

    for token in major_noise_tokens() {
        if markdown.contains(token) {
            grade = QualityGrade::Fail;
            findings.push(format!("major chrome leakage: {token}"));
        }
    }

    for token in minor_noise_tokens() {
        if markdown.contains(token) {
            grade = grade.max(QualityGrade::Warn);
            findings.push(format!("recoverable chrome leakage: {token}"));
        }
    }

    if markdown.contains("[](/") || markdown.contains("[](#") {
        grade = grade.max(QualityGrade::Warn);
        findings.push("empty-link artifacts remain in markdown".to_string());
    }

    if findings.is_empty() {
        findings.push("content looks clean".to_string());
    }

    (grade, findings)
}

fn major_noise_tokens() -> &'static [&'static str] {
    &[
        "CTRL K",
        "Copy page",
        "Skip to Content",
        "localStorage.getItem(",
        "document.querySelector(",
        "classList.toggle(",
        "__NEXT_DATA__",
    ]
}

fn minor_noise_tokens() -> &'static [&'static str] {
    &[
        "Was this page helpful?",
        "Thanks for your feedback!",
        "Edit this page",
        "Last updated on",
        "[¶](#",
        "Are you an LLM? View /llms.txt",
    ]
}

fn has_exported_page_for_url(export: &ExportResult, url: &str) -> bool {
    export
        .pages
        .iter()
        .any(|page| canonicalize_live_url(&page.page.url) == canonicalize_live_url(url))
        && export
            .output_dir
            .join(relative_output_path_for_url(url))
            .exists()
}

fn relative_output_path_for_url(url: &str) -> String {
    let parsed = Url::parse(&canonicalize_live_url(url)).expect("url should parse");
    if parsed.path().trim_matches('/').is_empty() {
        return "index.md".to_string();
    }

    format!("{}.md", parsed.path().trim_start_matches('/'))
}

fn load_targets() -> Vec<LiveTarget> {
    serde_json::from_str(&fs::read_to_string(targets_path()).expect("targets file should exist"))
        .expect("targets should deserialize")
}

fn find_target(name: &str) -> LiveTarget {
    load_targets()
        .into_iter()
        .find(|target| target.name == name)
        .unwrap_or_else(|| panic!("unknown target {name}"))
}

fn targets_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("live_targets.json")
}

fn is_setext_underline(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3 && trimmed.chars().all(|ch| ch == '=' || ch == '-')
}

fn canonicalize_live_url(url: &str) -> String {
    url.trim_end_matches('/').to_string()
}

fn benchmark_output_dir(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "docsite-to-md-live-benchmark-{name}-{}",
        now_epoch_nanos()
    ))
}

fn now_epoch() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_secs()
}

fn now_epoch_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should move forward")
        .as_nanos()
}
