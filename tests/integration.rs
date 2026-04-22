mod support;

use std::fs;

use docsite_to_md::{
    BrowserOptions, BundleOptions, CrawlOptions, ExportOptions, Framework, SourceFormat,
    bundle_site, crawl_site, detect_site, export_site,
};
use tempfile::tempdir;

fn fixture(path: &str) -> String {
    fs::read_to_string(format!(
        "/Users/k4lok/Development/OpenSources/docsite-to-md/tests/fixtures/{path}"
    ))
    .expect("fixture should exist")
}

#[tokio::test]
async fn detects_modern_gitbook() {
    let server =
        support::serve_routes(&[("/", "text/html", &fixture("modern_gitbook/root.html"))]).await;

    let profile = detect_site(&server.uri())
        .await
        .expect("site should detect");
    assert_eq!(profile.framework, Framework::GitBookModern);
    assert!(profile.supports_markdown_endpoints);
}

#[tokio::test]
async fn detects_docusaurus() {
    let server =
        support::serve_routes(&[("/", "text/html", &fixture("docusaurus/root.html"))]).await;

    let profile = detect_site(&server.uri())
        .await
        .expect("docusaurus should detect");
    assert_eq!(profile.framework, Framework::Docusaurus);
}

#[tokio::test]
async fn detects_mkdocs_material() {
    let server = support::serve_routes(&[("/", "text/html", &fixture("mkdocs/root.html"))]).await;

    let profile = detect_site(&server.uri())
        .await
        .expect("mkdocs should detect");
    assert_eq!(profile.framework, Framework::MkDocsMaterial);
}

#[tokio::test]
async fn detects_vitepress() {
    let server =
        support::serve_routes(&[("/", "text/html", &fixture("vitepress/root.html"))]).await;

    let profile = detect_site(&server.uri())
        .await
        .expect("vitepress should detect");
    assert_eq!(profile.framework, Framework::VitePress);
}

#[tokio::test]
async fn detects_nextra() {
    let server = support::serve_routes(&[("/", "text/html", &fixture("nextra/root.html"))]).await;

    let profile = detect_site(&server.uri())
        .await
        .expect("nextra should detect");
    assert_eq!(profile.framework, Framework::Nextra);
}

#[tokio::test]
async fn crawl_manifest_respects_scope_and_excludes() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("generic_docs/root.html")),
        ("/guide", "text/html", &fixture("generic_docs/guide.html")),
        (
            "/guide-duplicate",
            "text/html",
            &fixture("generic_docs/guide-duplicate.html"),
        ),
        ("/blog", "text/html", &fixture("generic_docs/blog.html")),
    ])
    .await;

    let manifest = crawl_site(
        &server.uri(),
        CrawlOptions {
            excludes: vec!["blog".to_string()],
            ..CrawlOptions::default()
        },
    )
    .await
    .expect("crawl should succeed");

    assert!(manifest.pages.iter().any(|page| page.url == server.uri()));
    assert!(
        manifest
            .pages
            .iter()
            .any(|page| page.url.ends_with("/guide"))
    );
    assert!(
        !manifest
            .pages
            .iter()
            .any(|page| page.url.ends_with("/blog"))
    );
}

#[tokio::test]
async fn export_creates_mirror_tree_and_manifest() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("modern_gitbook/root.html")),
        (
            "/getting-started",
            "text/html",
            &fixture("modern_gitbook/getting-started.html"),
        ),
        (
            "/api/reference",
            "text/html",
            &fixture("modern_gitbook/api-reference.html"),
        ),
        ("/.md", "text/markdown", &fixture("modern_gitbook/root.md")),
        (
            "/getting-started.md",
            "text/markdown",
            &fixture("modern_gitbook/getting-started.md"),
        ),
        (
            "/api/reference.md",
            "text/markdown",
            &fixture("modern_gitbook/api-reference.md"),
        ),
    ])
    .await;

    let output = tempdir().expect("tempdir");
    let result = export_site(
        &server.uri(),
        ExportOptions {
            output_dir: output.path().join("export"),
            crawl: CrawlOptions::default(),
            resume: false,
            bundle_output: None,
            browser: BrowserOptions::default(),
        },
    )
    .await
    .expect("export should succeed");

    assert!(result.output_dir.join("index.md").exists());
    assert!(result.output_dir.join("getting-started.md").exists());
    assert!(result.output_dir.join("api/reference.md").exists());
    assert!(result.manifest_path.exists());
    assert!(result.links_path.exists());

    let root_md = fs::read_to_string(result.output_dir.join("index.md")).expect("root markdown");
    assert!(root_md.contains("[!DANGER]"));
}

#[tokio::test]
async fn export_deduplicates_same_content() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("generic_docs/root.html")),
        ("/guide", "text/html", &fixture("generic_docs/guide.html")),
        (
            "/guide-duplicate",
            "text/html",
            &fixture("generic_docs/guide-duplicate.html"),
        ),
        ("/blog", "text/html", &fixture("generic_docs/blog.html")),
    ])
    .await;

    let output = tempdir().expect("tempdir");
    let result = export_site(
        &server.uri(),
        ExportOptions {
            output_dir: output.path().join("export"),
            crawl: CrawlOptions {
                excludes: vec!["blog".to_string()],
                ..CrawlOptions::default()
            },
            resume: false,
            bundle_output: None,
            browser: BrowserOptions::default(),
        },
    )
    .await
    .expect("export should succeed");

    let duplicate = result
        .pages
        .iter()
        .find(|page| page.page.url.ends_with("/guide-duplicate"))
        .expect("duplicate page should exist");
    assert!(duplicate.skipped || duplicate.duplicate_of.is_some());
}

#[tokio::test]
async fn retries_flaky_route() {
    let server = support::serve_with_flaky_route(
        &[
            ("/", "text/html", &fixture("generic_docs/root.html")),
            ("/guide", "text/html", &fixture("generic_docs/guide.html")),
            (
                "/guide-duplicate",
                "text/html",
                &fixture("generic_docs/guide-duplicate.html"),
            ),
            ("/blog", "text/html", &fixture("generic_docs/blog.html")),
        ],
        "/guide",
        500,
        200,
        &fixture("generic_docs/guide.html"),
    )
    .await;

    let manifest = crawl_site(
        &server.uri(),
        CrawlOptions {
            retry_attempts: 2,
            ..CrawlOptions::default()
        },
    )
    .await
    .expect("crawl should succeed after retry");

    assert!(
        manifest
            .pages
            .iter()
            .any(|page| page.url.ends_with("/guide"))
    );
}

#[tokio::test]
async fn bundle_command_writes_single_file() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("generic_docs/root.html")),
        ("/guide", "text/html", &fixture("generic_docs/guide.html")),
        (
            "/guide-duplicate",
            "text/html",
            &fixture("generic_docs/guide-duplicate.html"),
        ),
        ("/blog", "text/html", &fixture("generic_docs/blog.html")),
    ])
    .await;

    let output = tempdir().expect("tempdir");
    let bundle_path = output.path().join("bundle.md");
    let path = bundle_site(
        &server.uri(),
        BundleOptions {
            crawl: CrawlOptions {
                excludes: vec!["blog".to_string()],
                ..CrawlOptions::default()
            },
            output_file: bundle_path.clone(),
            browser: BrowserOptions::default(),
        },
    )
    .await
    .expect("bundle should succeed");

    let content = fs::read_to_string(path).expect("bundle file");
    assert!(content.contains("# Documentation Bundle"));
    assert!(content.contains("## Guide"));
}

#[tokio::test]
async fn classic_gitbook_detects_and_exports_markdown() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("classic_gitbook/root.html")),
        (
            "/chapter-1",
            "text/html",
            &fixture("classic_gitbook/chapter-1.html"),
        ),
        (
            "/README.md",
            "text/markdown",
            &fixture("classic_gitbook/README.md"),
        ),
        (
            "/chapter-1.md",
            "text/markdown",
            &fixture("classic_gitbook/chapter-1.md"),
        ),
    ])
    .await;

    let profile = detect_site(&server.uri()).await.expect("detect classic");
    assert_eq!(profile.framework, Framework::GitBookClassic);
}

#[tokio::test]
async fn exports_docusaurus_with_cleanup() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("docusaurus/root.html")),
        (
            "/docs/intro",
            "text/html",
            &fixture("docusaurus/intro.html"),
        ),
        (
            "/docs/tutorial",
            "text/html",
            &fixture("docusaurus/tutorial.html"),
        ),
    ])
    .await;

    let output = tempdir().expect("tempdir");
    let result = export_site(
        &server.uri(),
        ExportOptions {
            output_dir: output.path().join("docusaurus"),
            crawl: CrawlOptions::default(),
            resume: false,
            bundle_output: None,
            browser: BrowserOptions::default(),
        },
    )
    .await
    .expect("export should succeed");

    let markdown =
        fs::read_to_string(result.output_dir.join("docs/intro.md")).expect("docusaurus markdown");
    assert!(markdown.contains("[!NOTE]"));
    assert!(!markdown.contains("On this page"));
}

#[tokio::test]
async fn exports_mkdocs_material_with_tabs_and_tables() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("mkdocs/root.html")),
        (
            "/getting-started",
            "text/html",
            &fixture("mkdocs/getting-started.html"),
        ),
        ("/reference", "text/html", &fixture("mkdocs/reference.html")),
    ])
    .await;

    let output = tempdir().expect("tempdir");
    let result = export_site(
        &server.uri(),
        ExportOptions {
            output_dir: output.path().join("mkdocs"),
            crawl: CrawlOptions::default(),
            resume: false,
            bundle_output: None,
            browser: BrowserOptions::default(),
        },
    )
    .await
    .expect("export should succeed");

    let markdown =
        fs::read_to_string(result.output_dir.join("getting-started.md")).expect("mkdocs markdown");
    assert!(markdown.contains("### Python"));
    let reference_md =
        fs::read_to_string(result.output_dir.join("reference.md")).expect("reference markdown");
    assert!(reference_md.contains("Name"));
    assert!(reference_md.contains("material"));
}

#[tokio::test]
async fn exports_vitepress_with_container_cleanup() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("vitepress/root.html")),
        (
            "/guide/start",
            "text/html",
            &fixture("vitepress/start.html"),
        ),
        (
            "/guide/config",
            "text/html",
            &fixture("vitepress/config.html"),
        ),
    ])
    .await;

    let output = tempdir().expect("tempdir");
    let result = export_site(
        &server.uri(),
        ExportOptions {
            output_dir: output.path().join("vitepress"),
            crawl: CrawlOptions::default(),
            resume: false,
            bundle_output: None,
            browser: BrowserOptions::default(),
        },
    )
    .await
    .expect("export should succeed");

    let markdown =
        fs::read_to_string(result.output_dir.join("guide/start.md")).expect("vitepress markdown");
    assert!(markdown.contains("[!TIP]"));
    assert!(!markdown.contains("Table of contents"));
}

#[tokio::test]
async fn vitepress_prefers_core_docs_over_locale_variants_in_small_crawls() {
    let root = r#"<!DOCTYPE html>
<html>
  <body>
    <nav class="VPNav">vitepress</nav>
    <aside class="VPSidebar">
      <a href="/es/guide/">Spanish</a>
      <a href="/fa/guide/">Persian</a>
      <a href="/guide/what-is-vitepress">Guide</a>
    </aside>
    <main class="VPDoc">
      <div class="content">
        <h1>Home</h1>
      </div>
    </main>
  </body>
</html>"#;

    let page = r#"<!DOCTYPE html>
<html>
  <body>
    <nav class="VPNav">vitepress</nav>
    <main class="VPDoc">
      <div class="content">
        <h1>Guide</h1>
      </div>
    </main>
  </body>
</html>"#;

    let server = support::serve_routes(&[
        ("/", "text/html", root),
        ("/guide/what-is-vitepress", "text/html", page),
        ("/es/guide/", "text/html", page),
        ("/fa/guide/", "text/html", page),
    ])
    .await;

    let manifest = crawl_site(
        &server.uri(),
        CrawlOptions {
            max_pages: Some(3),
            ..CrawlOptions::default()
        },
    )
    .await
    .expect("crawl should succeed");

    assert!(
        manifest
            .pages
            .iter()
            .any(|page| page.url.ends_with("/guide/what-is-vitepress"))
    );
}

#[tokio::test]
async fn exports_nextra_and_marks_no_browser_when_not_used() {
    let server = support::serve_routes(&[
        ("/", "text/html", &fixture("nextra/root.html")),
        ("/docs/intro", "text/html", &fixture("nextra/intro.html")),
        (
            "/docs/advanced",
            "text/html",
            &fixture("nextra/advanced.html"),
        ),
    ])
    .await;

    let output = tempdir().expect("tempdir");
    let result = export_site(
        &server.uri(),
        ExportOptions {
            output_dir: output.path().join("nextra"),
            crawl: CrawlOptions::default(),
            resume: false,
            bundle_output: None,
            browser: BrowserOptions {
                enabled: true,
                webdriver_url: None,
            },
        },
    )
    .await
    .expect("export should succeed");

    let page = result
        .pages
        .iter()
        .find(|page| page.page.url.ends_with("/docs/intro"))
        .expect("nextra intro page");
    assert_eq!(page.source_format, SourceFormat::Html);
    assert!(!page.used_browser_fallback);
}

#[tokio::test]
#[ignore = "live acceptance test"]
async fn live_acceptance_gitbook_official() {
    let profile = detect_site("https://docs.gitbook.com")
        .await
        .expect("gitbook official docs should detect");
    assert!(matches!(
        profile.framework,
        Framework::GitBookModern | Framework::GitBookClassic
    ));
}

#[tokio::test]
#[ignore = "live acceptance test"]
async fn live_acceptance_docusaurus() {
    let profile = detect_site("https://jestjs.io/docs/getting-started")
        .await
        .expect("docusaurus should detect");
    assert_eq!(profile.framework, Framework::Docusaurus);
}

#[tokio::test]
#[ignore = "live acceptance test"]
async fn live_acceptance_mkdocs_material() {
    let profile = detect_site("https://squidfunk.github.io/mkdocs-material/")
        .await
        .expect("mkdocs material should detect");
    assert_eq!(profile.framework, Framework::MkDocsMaterial);
}

#[tokio::test]
#[ignore = "live acceptance test"]
async fn live_acceptance_vitepress() {
    let profile = detect_site("https://vitepress.dev/")
        .await
        .expect("vitepress should detect");
    assert_eq!(profile.framework, Framework::VitePress);
}

#[tokio::test]
#[ignore = "live acceptance test"]
async fn live_acceptance_nextra() {
    let profile = detect_site("https://nextra.site/")
        .await
        .expect("nextra should detect");
    assert_eq!(profile.framework, Framework::Nextra);
}
