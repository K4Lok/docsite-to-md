use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Framework {
    GitBookModern,
    GitBookClassic,
    GenericDocsFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SourceFormat {
    Markdown,
    Html,
    BrowserHtml,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SiteProfile {
    pub url: String,
    pub framework: Framework,
    pub extractor: String,
    pub supports_markdown_endpoints: bool,
    pub browser_fallback_recommended: bool,
    pub detection_signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageRef {
    pub url: String,
    pub normalized_url: String,
    pub title: Option<String>,
    pub depth: usize,
    pub parent_url: Option<String>,
    pub markdown_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlOptions {
    pub scope_prefix: Option<String>,
    pub excludes: Vec<String>,
    pub retry_attempts: usize,
    pub rate_limit_ms: u64,
    pub max_concurrency: usize,
    pub max_pages: Option<usize>,
}

impl Default for CrawlOptions {
    fn default() -> Self {
        Self {
            scope_prefix: None,
            excludes: Vec::new(),
            retry_attempts: 2,
            rate_limit_ms: 0,
            max_concurrency: 8,
            max_pages: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlManifest {
    pub site: SiteProfile,
    pub generated_at_epoch: u64,
    pub pages: Vec<PageRef>,
    pub skipped_urls: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserOptions {
    pub enabled: bool,
    pub webdriver_url: Option<String>,
}

impl Default for BrowserOptions {
    fn default() -> Self {
        Self {
            enabled: false,
            webdriver_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportOptions {
    pub output_dir: PathBuf,
    pub crawl: CrawlOptions,
    pub resume: bool,
    pub bundle_output: Option<PathBuf>,
    pub browser: BrowserOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleOptions {
    pub crawl: CrawlOptions,
    pub output_file: PathBuf,
    pub browser: BrowserOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportedPage {
    pub page: PageRef,
    pub output_file: String,
    pub title: Option<String>,
    pub source_format: SourceFormat,
    pub content_hash: Option<String>,
    pub skipped: bool,
    pub duplicate_of: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageError {
    pub url: String,
    pub phase: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportResult {
    pub site: SiteProfile,
    pub output_dir: PathBuf,
    pub manifest_path: PathBuf,
    pub links_path: PathBuf,
    pub bundle_path: Option<PathBuf>,
    pub pages: Vec<ExportedPage>,
    pub errors: Vec<PageError>,
}
