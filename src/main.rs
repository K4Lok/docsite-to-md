use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use docsite_to_md::{
    BrowserOptions, BundleOptions, CrawlOptions, ExportOptions, bundle_site, crawl_site,
    detect_site, export_site,
};

#[derive(Parser)]
#[command(name = "docsite-to-md")]
#[command(about = "Export documentation sites to Markdown", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Detect {
        url: String,
    },
    Crawl {
        url: String,
        #[command(flatten)]
        common: CommonArgs,
    },
    Export {
        url: String,
        #[arg(short, long, default_value = "output")]
        output_dir: PathBuf,
        #[arg(long)]
        resume: bool,
        #[arg(long)]
        bundle_output: Option<PathBuf>,
        #[arg(long)]
        browser_fallback: bool,
        #[arg(long)]
        webdriver_url: Option<String>,
        #[command(flatten)]
        common: CommonArgs,
    },
    Bundle {
        url: String,
        #[arg(short, long, default_value = "bundle.md")]
        output: PathBuf,
        #[arg(long)]
        browser_fallback: bool,
        #[arg(long)]
        webdriver_url: Option<String>,
        #[command(flatten)]
        common: CommonArgs,
    },
}

#[derive(Args, Clone)]
struct CommonArgs {
    #[arg(long)]
    scope_prefix: Option<String>,
    #[arg(long)]
    exclude: Vec<String>,
    #[arg(long, default_value_t = 2)]
    retries: usize,
    #[arg(long, default_value_t = 0)]
    rate_limit_ms: u64,
    #[arg(long, default_value_t = 8)]
    concurrency: usize,
    #[arg(long)]
    max_pages: Option<usize>,
}

impl From<CommonArgs> for CrawlOptions {
    fn from(value: CommonArgs) -> Self {
        CrawlOptions {
            scope_prefix: value.scope_prefix,
            excludes: value.exclude,
            retry_attempts: value.retries,
            rate_limit_ms: value.rate_limit_ms,
            max_concurrency: value.concurrency,
            max_pages: value.max_pages,
        }
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Detect { url } => detect_site(&url)
            .await
            .and_then(|profile| serde_json::to_string_pretty(&profile).map_err(Into::into)),
        Commands::Crawl { url, common } => crawl_site(&url, common.into())
            .await
            .and_then(|manifest| serde_json::to_string_pretty(&manifest).map_err(Into::into)),
        Commands::Export {
            url,
            output_dir,
            resume,
            bundle_output,
            browser_fallback,
            webdriver_url,
            common,
        } => {
            let result = export_site(
                &url,
                ExportOptions {
                    output_dir,
                    crawl: common.into(),
                    resume,
                    bundle_output,
                    browser: BrowserOptions {
                        enabled: browser_fallback,
                        webdriver_url,
                    },
                },
            )
            .await;

            result.and_then(|result| serde_json::to_string_pretty(&result).map_err(Into::into))
        }
        Commands::Bundle {
            url,
            output,
            browser_fallback,
            webdriver_url,
            common,
        } => {
            let result = bundle_site(
                &url,
                BundleOptions {
                    crawl: common.into(),
                    output_file: output,
                    browser: BrowserOptions {
                        enabled: browser_fallback,
                        webdriver_url,
                    },
                },
            )
            .await;

            result.map(|path| format!("{{\n  \"bundle_path\": \"{}\"\n}}", path.display()))
        }
    };

    match result {
        Ok(output) => println!("{output}"),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(1);
        }
    }
}
