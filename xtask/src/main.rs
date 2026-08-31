//! Repository maintenance commands.

use std::{
    collections::{BTreeMap, HashSet},
    env,
    error::Error,
    fmt::Write as _,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use futures_util::{StreamExt as _, stream};
use reqwest::{Client, StatusCode, Url};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use tokio::{fs, time::sleep};

const DEFAULT_BASE_URL: &str = "https://upstream.rrepo.dev/cran";
const DEFAULT_CONCURRENCY: usize = 256;
const MAX_DESCRIPTION_BYTES: usize = 2 * 1024 * 1024;
const ATTEMPTS: usize = 8;

type BoxError = Box<dyn Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), BoxError> {
    let mut arguments = env::args().skip(1);
    match arguments.next().as_deref() {
        Some("cran-snapshot") => {
            let concurrency = arguments
                .next()
                .map_or(Ok(DEFAULT_CONCURRENCY), |value| value.parse())?;
            if concurrency == 0 {
                return Err("concurrency must be greater than zero".into());
            }
            snapshot(DEFAULT_BASE_URL, concurrency).await
        }
        _ => Err(format!(
            "usage: cargo run -p xtask -- cran-snapshot [concurrency] (default {DEFAULT_CONCURRENCY})"
        )
        .into()),
    }
}

#[allow(clippy::too_many_lines)]
async fn snapshot(base_url: &str, concurrency: usize) -> Result<(), BoxError> {
    let workspace = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("xtask must be inside the workspace")?
        .to_owned();
    let corpus_parent = workspace.join("testdata/cran");
    let staging = corpus_parent.join("latest.tmp");
    let destination = corpus_parent.join("latest");

    fs::create_dir_all(staging.join("packages")).await?;
    let stale_failures = staging.join("failures.json");
    if stale_failures.exists() {
        fs::remove_file(stale_failures).await?;
    }

    let client = Client::builder()
        .user_agent(concat!(
            "r-metadata-rs-corpus/",
            env!("CARGO_PKG_VERSION"),
            " (https://github.com/rrepo-org/r-metadata-rs)"
        ))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()?;
    let index_url = format!("{}/packages", base_url.trim_end_matches('/'));
    eprintln!("fetching {index_url}");
    let index_bytes = fetch_bytes(&client, Url::parse(&index_url)?, 64 * 1024 * 1024).await?;
    let index: PackagesResponse = serde_json::from_slice(&index_bytes)?;
    let packages = normalize_packages(index.packages)?;
    eprintln!(
        "fetching {} latest DESCRIPTION files with concurrency {concurrency}",
        packages.len()
    );

    fs::write(staging.join("packages-index.json"), &index_bytes).await?;
    let base = Url::parse(base_url)?;
    let staging = Arc::new(staging);
    let client = Arc::new(client);
    let total = packages.len();
    let mut completed = 0_usize;
    let mut entries = Vec::with_capacity(total);
    let mut failures = Vec::new();
    let mut downloads = stream::iter(packages.into_values().map(|package| {
        let client = Arc::clone(&client);
        let staging = Arc::clone(&staging);
        let base = base.clone();
        async move {
            let result = fetch_description(&client, &base, &staging, &package).await;
            (package, result)
        }
    }))
    .buffer_unordered(concurrency);

    while let Some((package, result)) = downloads.next().await {
        completed += 1;
        match result {
            Ok(entry) => entries.push(entry),
            Err(error) => failures.push(FetchFailure {
                package: package.name,
                version: package.latest_version,
                error: error.to_string(),
            }),
        }
        if completed.is_multiple_of(1_000) || completed == total {
            eprintln!("completed {completed}/{total}, failures {}", failures.len());
        }
    }

    entries.sort_unstable_by(|left, right| left.package.cmp(&right.package));
    failures.sort_unstable_by(|left, right| left.package.cmp(&right.package));
    prune_stale_fixtures(&staging, &entries)?;
    if !failures.is_empty() {
        fs::write(
            staging.join("failures.json"),
            serde_json::to_vec_pretty(&failures)?,
        )
        .await?;
    }

    let generated_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let manifest = SnapshotManifest {
        schema_version: 1,
        repository: index.repository_slug,
        base_url: base_url.to_owned(),
        generated_at_unix: generated_at,
        index_sha256: sha256(&index_bytes),
        package_count: total,
        fixture_count: entries.len(),
        failure_count: failures.len(),
        entries,
        failures,
    };
    fs::write(
        staging.join("snapshot.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )
    .await?;
    verify_snapshot(&staging, &manifest).await?;

    if destination.exists() {
        fs::remove_dir_all(&destination).await?;
    }
    fs::rename(staging.as_ref(), &destination).await?;
    eprintln!(
        "published {} verified fixtures ({} unavailable) to {}",
        manifest.fixture_count,
        manifest.failure_count,
        destination.display()
    );
    Ok(())
}

fn normalize_packages(
    packages: Vec<PackageSummary>,
) -> Result<BTreeMap<String, PackageSummary>, BoxError> {
    let mut normalized = BTreeMap::new();
    for package in packages {
        validate_segment("package", &package.name)?;
        validate_segment("version", &package.latest_version)?;
        if normalized.insert(package.name.clone(), package).is_some() {
            return Err("package index contains duplicate names".into());
        }
    }
    Ok(normalized)
}

fn validate_segment(kind: &str, value: &str) -> Result<(), BoxError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || !value.is_ascii()
        || value
            .bytes()
            .any(|byte| !(byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_')))
    {
        return Err(format!("unsafe {kind} path segment {value:?}").into());
    }
    Ok(())
}

async fn fetch_description(
    client: &Client,
    base: &Url,
    staging: &Path,
    package: &PackageSummary,
) -> Result<SnapshotEntry, BoxError> {
    let fixture_directory = format!(
        "{}--{}",
        package.name,
        &sha256(package.name.as_bytes())[..12]
    );
    let relative_path = format!(
        "packages/{}/{}/DESCRIPTION",
        fixture_directory, package.latest_version
    );
    let path = staging.join(&relative_path);
    let legacy_path = staging.join(format!(
        "packages/{}/{}/DESCRIPTION",
        package.name, package.latest_version
    ));
    if !path.exists() && legacy_path.exists() {
        let parent = path.parent().ok_or("DESCRIPTION path has no parent")?;
        fs::create_dir_all(parent).await?;
        fs::rename(&legacy_path, &path).await?;
    }
    if path.exists() {
        let bytes = fs::read(&path).await?;
        return Ok(snapshot_entry(package, relative_path, &bytes));
    }

    let mut description_url = base.clone();
    description_url
        .path_segments_mut()
        .map_err(|()| "rrepo base URL cannot be a base")?
        .pop_if_empty()
        .extend([
            "packages",
            &package.name,
            "versions",
            &package.latest_version,
            "description",
        ]);
    let bytes = fetch_bytes(client, description_url, MAX_DESCRIPTION_BYTES).await?;
    let parent = path.parent().ok_or("DESCRIPTION path has no parent")?;
    fs::create_dir_all(parent).await?;
    let temporary = parent.join("DESCRIPTION.tmp");
    fs::write(&temporary, &bytes).await?;
    fs::rename(temporary, &path).await?;
    Ok(snapshot_entry(package, relative_path, &bytes))
}

fn snapshot_entry(package: &PackageSummary, relative_path: String, bytes: &[u8]) -> SnapshotEntry {
    SnapshotEntry {
        package: package.name.clone(),
        version: package.latest_version.clone(),
        path: relative_path,
        bytes: bytes.len(),
        sha256: sha256(bytes),
    }
}

fn prune_stale_fixtures(root: &Path, entries: &[SnapshotEntry]) -> Result<(), BoxError> {
    let expected = entries
        .iter()
        .map(|entry| root.join(&entry.path))
        .collect::<HashSet<_>>();
    let packages = root.join("packages");
    for entry in walkdir::WalkDir::new(&packages)
        .min_depth(1)
        .contents_first(true)
    {
        let entry = entry?;
        if entry.file_type().is_file() && !expected.contains(entry.path()) {
            std::fs::remove_file(entry.path())?;
        } else if entry.file_type().is_dir() && std::fs::read_dir(entry.path())?.next().is_none() {
            std::fs::remove_dir(entry.path())?;
        }
    }
    Ok(())
}

async fn fetch_bytes(client: &Client, url: Url, limit: usize) -> Result<Vec<u8>, BoxError> {
    let mut last_error = String::new();
    for attempt in 0..ATTEMPTS {
        match client.get(url.clone()).send().await {
            Ok(response) if response.status().is_success() => {
                let bytes = response.bytes().await?;
                if bytes.len() > limit {
                    return Err(format!("response exceeded {limit} bytes: {url}").into());
                }
                return Ok(bytes.to_vec());
            }
            Ok(response) => {
                let status = response.status();
                last_error = format!("HTTP {status} from {url}");
                if !retryable_status(status) {
                    return Err(last_error.into());
                }
            }
            Err(error) => last_error = format!("request failed for {url}: {error}"),
        }
        let delay = u64::try_from(attempt + 1).unwrap_or(10).min(10);
        sleep(Duration::from_secs(delay)).await;
    }
    Err(last_error.into())
}

const fn retryable_status(status: StatusCode) -> bool {
    matches!(
        status,
        StatusCode::NOT_FOUND
            | StatusCode::REQUEST_TIMEOUT
            | StatusCode::TOO_MANY_REQUESTS
            | StatusCode::INTERNAL_SERVER_ERROR
            | StatusCode::BAD_GATEWAY
            | StatusCode::SERVICE_UNAVAILABLE
            | StatusCode::GATEWAY_TIMEOUT
    )
}

async fn verify_snapshot(root: &Path, manifest: &SnapshotManifest) -> Result<(), BoxError> {
    if manifest.package_count != manifest.entries.len() + manifest.failures.len()
        || manifest.fixture_count != manifest.entries.len()
        || manifest.failure_count != manifest.failures.len()
    {
        return Err("manifest counts do not match entries and failures".into());
    }
    for entry in &manifest.entries {
        let bytes = fs::read(root.join(&entry.path)).await?;
        if bytes.len() != entry.bytes || sha256(&bytes) != entry.sha256 {
            return Err(format!("fixture verification failed for {}", entry.package).into());
        }
    }
    Ok(())
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        write!(output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackagesResponse {
    repository_slug: String,
    packages: Vec<PackageSummary>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageSummary {
    name: String,
    latest_version: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotManifest {
    schema_version: u32,
    repository: String,
    base_url: String,
    generated_at_unix: u64,
    index_sha256: String,
    package_count: usize,
    fixture_count: usize,
    failure_count: usize,
    entries: Vec<SnapshotEntry>,
    failures: Vec<FetchFailure>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotEntry {
    package: String,
    version: String,
    path: String,
    bytes: usize,
    sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FetchFailure {
    package: String,
    version: String,
    error: String,
}
