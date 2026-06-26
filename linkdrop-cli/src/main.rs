use anyhow::Context;
use clap::{Parser, Subcommand};
use linkdrop_core::{config::Config, ttl::parse_ttl};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "linkdrop", version, about = "Push HTML and get a shareable link")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, env = "LINKDROP_URL", global = true)]
    url: Option<String>,

    #[arg(long, env = "LINKDROP_TOKEN", global = true)]
    token: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Upload an HTML file or stdin content
    Push {
        /// Path to an HTML file (use --stdin to read from stdin instead)
        file: Option<PathBuf>,

        #[arg(long)]
        stdin: bool,

        #[arg(long)]
        slug: Option<String>,

        #[arg(long)]
        force: bool,

        #[arg(long)]
        ttl: Option<String>,
    },

    /// Delete a page by slug
    Delete { slug: String },

    /// List uploaded pages
    List,
}

#[derive(Deserialize)]
struct PageResponse {
    slug: String,
    url: String,
    size_bytes: i64,
    expires_at: Option<String>,
}

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let (url, token) = resolve_credentials(&cli)?;

    let client = reqwest::Client::new();
    let base = url.trim_end_matches('/');

    match cli.command {
        Commands::Push {
            file,
            stdin,
            slug,
            force,
            ttl,
        } => {
            let html = read_html_input(file, stdin).await?;
            if let Some(ttl_str) = &ttl {
                parse_ttl(ttl_str).context("invalid ttl")?;
            }

            let mut body = serde_json::json!({
                "html": html,
                "force": force,
            });
            if let Some(slug) = slug {
                body["slug"] = serde_json::Value::String(slug);
            }
            if let Some(ttl) = ttl {
                body["ttl"] = serde_json::Value::String(ttl);
            }

            let response = client
                .post(format!("{base}/api/pages"))
                .header("Authorization", format!("Bearer {token}"))
                .json(&body)
                .send()
                .await
                .context("request failed")?;

            handle_page_response(response).await?;
        }
        Commands::Delete { slug } => {
            let response = client
                .delete(format!("{base}/api/pages/{slug}"))
                .header("Authorization", format!("Bearer {token}"))
                .send()
                .await
                .context("request failed")?;

            if response.status().is_success() {
                println!("deleted {slug}");
            } else {
                handle_error_response(response).await?;
            }
        }
        Commands::List => {
            let response = client
                .get(format!("{base}/api/pages"))
                .header("Authorization", format!("Bearer {token}"))
                .send()
                .await
                .context("request failed")?;

            if !response.status().is_success() {
                handle_error_response(response).await?;
                return Ok(());
            }

            let pages: Vec<PageResponse> = response.json().await?;
            if pages.is_empty() {
                println!("no pages");
                return Ok(());
            }

            for page in pages {
                let expires = page.expires_at.as_deref().unwrap_or("never");
                println!(
                    "{}  {}  {} bytes  expires: {}",
                    page.slug, page.url, page.size_bytes, expires
                );
            }
        }
    }

    Ok(())
}

fn resolve_credentials(cli: &Cli) -> anyhow::Result<(String, String)> {
    let file_config = Config::load();

    let url = cli
        .url
        .clone()
        .or(file_config.url)
        .filter(|s| !s.is_empty())
        .context("LINKDROP_URL is not set (env, config file, or --url)")?;

    let token = cli
        .token
        .clone()
        .or(file_config.token)
        .filter(|s| !s.is_empty())
        .context("LINKDROP_TOKEN is not set (env, config file, or --token)")?;

    Ok((url, token))
}

async fn read_html_input(file: Option<PathBuf>, stdin: bool) -> anyhow::Result<String> {
    if stdin {
        use tokio::io::{AsyncReadExt, BufReader};
        let mut reader = BufReader::new(tokio::io::stdin());
        let mut buf = Vec::new();
        reader.read_to_end(&mut buf).await?;
        return Ok(String::from_utf8(buf).context("stdin is not valid utf-8")?);
    }

    let path = file.context("provide a file path or use --stdin")?;
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    Ok(content)
}

async fn handle_page_response(response: reqwest::Response) -> anyhow::Result<()> {
    if response.status().is_success() {
        let page: PageResponse = response.json().await?;
        println!("{}", page.url);
        return Ok(());
    }
    handle_error_response(response).await?;
    Ok(())
}

async fn handle_error_response(response: reqwest::Response) -> anyhow::Result<()> {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if let Ok(err) = serde_json::from_str::<ErrorResponse>(&body) {
        anyhow::bail!("{status}: {err}", err = err.error);
    }
    anyhow::bail!("{status}: {body}");
}
