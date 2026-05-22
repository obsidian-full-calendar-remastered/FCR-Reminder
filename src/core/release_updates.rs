use chrono::Utc;
use reqwest::header::{ACCEPT, ETAG, IF_NONE_MATCH, USER_AGENT};
use semver::Version;
use serde::{Deserialize, Serialize};
use std::fs::{create_dir_all, File};
use std::io::{Read, Write};
use std::path::PathBuf;
use std::time::Duration;

const RELEASE_CHECK_INTERVAL_SECONDS: i64 = 7 * 24 * 60 * 60;
const INITIAL_NETWORK_RETRY_SECONDS: i64 = 60 * 60;
const FOLLOWUP_NETWORK_RETRY_SECONDS: i64 = 3 * 24 * 60 * 60;
const RELEASE_REQUEST_TIMEOUT_SECONDS: u64 = 4;
const RELEASE_CONNECT_TIMEOUT_SECONDS: u64 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ReleaseInfo {
    pub version: String,
    pub name: String,
    pub html_url: String,
    pub published_at_rfc3339: String,
    pub published_at_epoch: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateStateSnapshot {
    pub current_version: String,
    pub update_available: bool,
    pub latest_release: Option<ReleaseInfo>,
    pub last_checked_at_epoch: i64,
    pub next_check_at_epoch: i64,
    pub last_notified_version: Option<String>,
    pub last_error: Option<String>,
    pub releases_page_url: String,
    pub status_label: String,
    pub menu_label: String,
}

impl UpdateStateSnapshot {
    pub fn unavailable(reason: impl Into<String>) -> Self {
        let now_epoch = Utc::now().timestamp();
        let releases_page_url = format!("{}/releases/latest", env!("CARGO_PKG_REPOSITORY").trim_end_matches('/'));
        let mut snapshot = Self::empty(env!("CARGO_PKG_VERSION").to_string(), releases_page_url, now_epoch);
        snapshot.last_error = Some(reason.into());
        snapshot.status_label = format!(
            "Update check unavailable: {}",
            snapshot.last_error.clone().unwrap_or_default()
        );
        snapshot.menu_label = "Release check unavailable".to_string();
        snapshot
    }

    fn empty(current_version: String, releases_page_url: String, now_epoch: i64) -> Self {
        Self {
            current_version,
            update_available: false,
            latest_release: None,
            last_checked_at_epoch: 0,
            next_check_at_epoch: now_epoch,
            last_notified_version: None,
            last_error: None,
            releases_page_url,
            status_label: "Update status unavailable".to_string(),
            menu_label: "Checking for updates...".to_string(),
        }
    }

    pub fn action_url(&self) -> String {
        self.latest_release
            .as_ref()
            .map(|release| {
                if release.html_url.trim().is_empty() {
                    self.releases_page_url.clone()
                } else {
                    release.html_url.clone()
                }
            })
            .unwrap_or_else(|| self.releases_page_url.clone())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct UpdateCheckCache {
    last_checked_at_epoch: i64,
    next_check_at_epoch: i64,
    etag: Option<String>,
    latest_release: Option<ReleaseInfo>,
    update_available: bool,
    last_notified_version: Option<String>,
    last_error: Option<String>,
    consecutive_network_failures: u32,
}

#[derive(Debug, Clone)]
pub struct UpdateRefreshResult {
    pub snapshot: UpdateStateSnapshot,
    pub should_notify: Option<ReleaseInfo>,
}

#[derive(Debug, Deserialize)]
struct GitHubLatestRelease {
    tag_name: String,
    name: String,
    html_url: String,
    published_at: Option<String>,
    draft: bool,
    prerelease: bool,
}

pub struct ReleaseUpdateService {
    client: reqwest::Client,
    cache_path: PathBuf,
    current_version_raw: String,
    current_version: Version,
    repository_url: String,
    releases_page_url: String,
    latest_api_url: String,
}

impl ReleaseUpdateService {
    pub fn new() -> Result<Self, String> {
        let repository_url = env!("CARGO_PKG_REPOSITORY").trim_end_matches('/').to_string();
        let cache_path = cache_path()?;
        let latest_api_url = build_latest_api_url(&repository_url)?;
        let current_version_raw = env!("CARGO_PKG_VERSION").to_string();
        Self::from_parts(
            build_client(
                Duration::from_secs(RELEASE_REQUEST_TIMEOUT_SECONDS),
                Duration::from_secs(RELEASE_CONNECT_TIMEOUT_SECONDS),
            )?,
            cache_path,
            current_version_raw,
            repository_url,
            latest_api_url,
        )
    }

    pub fn load_snapshot(&self) -> UpdateStateSnapshot {
        let now_epoch = Utc::now().timestamp();
        let cache = self.load_cache().unwrap_or_default();
        self.snapshot_from_cache(cache, now_epoch)
    }

    pub async fn refresh_if_due(&self, force: bool) -> UpdateRefreshResult {
        let now_epoch = Utc::now().timestamp();
        let mut cache = self.load_cache().unwrap_or_default();

        if !force && cache.next_check_at_epoch > now_epoch {
            return UpdateRefreshResult {
                snapshot: self.snapshot_from_cache(cache, now_epoch),
                should_notify: None,
            };
        }

        let mut request = self
            .client
            .get(&self.latest_api_url)
            .header(USER_AGENT, "fcr-reminder-update-checker")
            .header(ACCEPT, "application/vnd.github+json");

        if let Some(etag) = cache.etag.as_deref() {
            request = request.header(IF_NONE_MATCH, etag);
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                cache.last_checked_at_epoch = now_epoch;
                cache.consecutive_network_failures = cache.consecutive_network_failures.saturating_add(1);
                cache.next_check_at_epoch = now_epoch + network_retry_delay_seconds(cache.consecutive_network_failures);
                cache.last_error = Some(format!("GitHub release check failed: {}", error));
                let _ = self.save_cache(&cache);
                return UpdateRefreshResult {
                    snapshot: self.snapshot_from_cache(cache, now_epoch),
                    should_notify: None,
                };
            }
        };

        cache.consecutive_network_failures = 0;

        if response.status() == reqwest::StatusCode::NOT_MODIFIED {
            cache.last_checked_at_epoch = now_epoch;
            cache.next_check_at_epoch = now_epoch + RELEASE_CHECK_INTERVAL_SECONDS;
            cache.last_error = None;
            if let Some(etag) = response.headers().get(ETAG).and_then(|value| value.to_str().ok()) {
                cache.etag = Some(etag.to_string());
            }
            let _ = self.save_cache(&cache);
            return UpdateRefreshResult {
                snapshot: self.snapshot_from_cache(cache, now_epoch),
                should_notify: None,
            };
        }

        if !response.status().is_success() {
            cache.last_checked_at_epoch = now_epoch;
            cache.next_check_at_epoch = now_epoch + RELEASE_CHECK_INTERVAL_SECONDS;
            cache.last_error = Some(format!("GitHub release check returned HTTP {}", response.status()));
            let _ = self.save_cache(&cache);
            return UpdateRefreshResult {
                snapshot: self.snapshot_from_cache(cache, now_epoch),
                should_notify: None,
            };
        }

        let etag = response
            .headers()
            .get(ETAG)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string());

        let payload = match response.json::<GitHubLatestRelease>().await {
            Ok(payload) => payload,
            Err(error) => {
                cache.last_checked_at_epoch = now_epoch;
                cache.next_check_at_epoch = now_epoch + RELEASE_CHECK_INTERVAL_SECONDS;
                cache.last_error = Some(format!("Failed to parse GitHub release payload: {}", error));
                let _ = self.save_cache(&cache);
                return UpdateRefreshResult {
                    snapshot: self.snapshot_from_cache(cache, now_epoch),
                    should_notify: None,
                };
            }
        };

        cache.last_checked_at_epoch = now_epoch;
        cache.next_check_at_epoch = now_epoch + RELEASE_CHECK_INTERVAL_SECONDS;
        cache.etag = etag;
        cache.last_error = None;

        if payload.draft || payload.prerelease {
            cache.latest_release = None;
            cache.update_available = false;
            let _ = self.save_cache(&cache);
            return UpdateRefreshResult {
                snapshot: self.snapshot_from_cache(cache, now_epoch),
                should_notify: None,
            };
        }

        let release_info = match self.release_info_from_payload(payload) {
            Ok(info) => info,
            Err(error) => {
                cache.last_error = Some(error);
                let _ = self.save_cache(&cache);
                return UpdateRefreshResult {
                    snapshot: self.snapshot_from_cache(cache, now_epoch),
                    should_notify: None,
                };
            }
        };

        let update_available = is_newer_version(&release_info.version, &self.current_version);
        cache.latest_release = Some(release_info.clone());
        cache.update_available = update_available;
        let should_notify = if update_available
            && cache.last_notified_version.as_deref() != Some(release_info.version.as_str())
        {
            Some(release_info)
        } else {
            None
        };
        let _ = self.save_cache(&cache);

        UpdateRefreshResult {
            snapshot: self.snapshot_from_cache(cache, now_epoch),
            should_notify,
        }
    }

    pub fn mark_notified(&self, version: &str) {
        let mut cache = self.load_cache().unwrap_or_default();
        cache.last_notified_version = Some(version.to_string());
        let _ = self.save_cache(&cache);
    }

    pub fn releases_page_url(&self) -> &str {
        &self.releases_page_url
    }

    pub fn repository_url(&self) -> &str {
        &self.repository_url
    }

    fn from_parts(
        client: reqwest::Client,
        cache_path: PathBuf,
        current_version_raw: String,
        repository_url: String,
        latest_api_url: String,
    ) -> Result<Self, String> {
        let current_version = Version::parse(&current_version_raw)
            .map_err(|error| format!("Failed to parse current package version: {}", error))?;
        let releases_page_url = format!("{}/releases/latest", repository_url.trim_end_matches('/'));

        Ok(Self {
            client,
            cache_path,
            current_version_raw,
            current_version,
            repository_url,
            releases_page_url,
            latest_api_url,
        })
    }

    fn release_info_from_payload(&self, payload: GitHubLatestRelease) -> Result<ReleaseInfo, String> {
        let normalized_version = normalize_release_tag(&payload.tag_name)?;
        let published_at_epoch = payload
            .published_at
            .as_deref()
            .and_then(parse_rfc3339_epoch)
            .unwrap_or_default();

        Ok(ReleaseInfo {
            version: normalized_version,
            name: if payload.name.trim().is_empty() {
                payload.tag_name
            } else {
                payload.name
            },
            html_url: payload.html_url,
            published_at_rfc3339: payload.published_at.unwrap_or_default(),
            published_at_epoch,
        })
    }

    fn snapshot_from_cache(&self, cache: UpdateCheckCache, now_epoch: i64) -> UpdateStateSnapshot {
        let mut snapshot = UpdateStateSnapshot::empty(
            self.current_version_raw.clone(),
            self.releases_page_url.clone(),
            now_epoch,
        );

        snapshot.last_checked_at_epoch = cache.last_checked_at_epoch;
        snapshot.next_check_at_epoch = if cache.next_check_at_epoch > 0 {
            cache.next_check_at_epoch
        } else {
            now_epoch
        };
        snapshot.last_notified_version = cache.last_notified_version;
        snapshot.last_error = cache.last_error.clone();
        snapshot.latest_release = cache.latest_release.clone();
        snapshot.update_available = cache.update_available;

        if let Some(error) = cache.last_error {
            snapshot.status_label = format!("Update check failed: {}", error);
            snapshot.menu_label = "Release check unavailable".to_string();
            return snapshot;
        }

        if let Some(release) = snapshot.latest_release.as_ref() {
            if snapshot.update_available {
                snapshot.status_label = format!(
                    "Update available: {} (current {})",
                    release.version, snapshot.current_version
                );
                snapshot.menu_label = format!("Update available: {}", release.version);
            } else {
                snapshot.status_label = format!(
                    "You are up to date on {}",
                    snapshot.current_version
                );
                snapshot.menu_label = "No updates available".to_string();
            }
        } else if snapshot.last_checked_at_epoch == 0 {
            snapshot.status_label = "Update check pending".to_string();
            snapshot.menu_label = "Checking for updates...".to_string();
        } else {
            snapshot.status_label = "No published release information available".to_string();
            snapshot.menu_label = "No updates available".to_string();
        }

        snapshot
    }

    fn load_cache(&self) -> Result<UpdateCheckCache, String> {
        if !self.cache_path.exists() {
            return Ok(UpdateCheckCache::default());
        }

        let mut file = File::open(&self.cache_path)
            .map_err(|error| format!("Failed to open update cache: {}", error))?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)
            .map_err(|error| format!("Failed to read update cache: {}", error))?;

        if contents.trim().is_empty() {
            return Ok(UpdateCheckCache::default());
        }

        serde_json::from_str(&contents)
            .map_err(|error| format!("Failed to parse update cache: {}", error))
    }

    fn save_cache(&self, cache: &UpdateCheckCache) -> Result<(), String> {
        if let Some(parent) = self.cache_path.parent() {
            create_dir_all(parent)
                .map_err(|error| format!("Failed to create update cache directory: {}", error))?;
        }

        let payload = serde_json::to_string_pretty(cache)
            .map_err(|error| format!("Failed to serialize update cache: {}", error))?;

        let mut file = File::create(&self.cache_path)
            .map_err(|error| format!("Failed to create update cache: {}", error))?;
        file.write_all(payload.as_bytes())
            .map_err(|error| format!("Failed to write update cache: {}", error))
    }
}

fn cache_path() -> Result<PathBuf, String> {
    crate::core::get_app_dir()
        .map(|dir| dir.join("update-state.json"))
        .ok_or_else(|| "Could not determine application data directory for update cache".to_string())
}

fn build_client(timeout: Duration, connect_timeout: Duration) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(timeout)
        .connect_timeout(connect_timeout)
        .build()
        .map_err(|error| format!("Failed to build HTTP client: {}", error))
}

fn network_retry_delay_seconds(consecutive_network_failures: u32) -> i64 {
    if consecutive_network_failures <= 1 {
        INITIAL_NETWORK_RETRY_SECONDS
    } else {
        FOLLOWUP_NETWORK_RETRY_SECONDS
    }
}

fn build_latest_api_url(repository_url: &str) -> Result<String, String> {
    let parsed = url::Url::parse(repository_url)
        .map_err(|error| format!("Failed to parse repository URL: {}", error))?;
    let segments: Vec<&str> = parsed
        .path_segments()
        .ok_or_else(|| "Repository URL is missing path segments".to_string())?
        .filter(|segment| !segment.is_empty())
        .collect();

    if segments.len() < 2 {
        return Err("Repository URL must include owner and repository name".to_string());
    }

    Ok(format!(
        "https://api.github.com/repos/{}/{}/releases/latest",
        segments[0], segments[1]
    ))
}

fn normalize_release_tag(tag: &str) -> Result<String, String> {
    let normalized = tag.trim().trim_start_matches('v').trim_start_matches('V');
    Version::parse(normalized)
        .map(|version| version.to_string())
        .map_err(|error| format!("Invalid release tag '{}': {}", tag, error))
}

fn is_newer_version(candidate: &str, current: &Version) -> bool {
    Version::parse(candidate)
        .map(|version| version > *current)
        .unwrap_or(false)
}

fn parse_rfc3339_epoch(value: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|timestamp| timestamp.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    use std::thread;

    fn temp_cache_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "fcr-reminder-update-tests-{}-{}-{}.json",
            name,
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    fn test_service(
        name: &str,
        latest_api_url: String,
        current_version: &str,
        timeout_ms: u64,
    ) -> ReleaseUpdateService {
        let client = build_client(
            Duration::from_millis(timeout_ms),
            Duration::from_millis(timeout_ms.min(250)),
        )
        .unwrap();

        ReleaseUpdateService::from_parts(
            client,
            temp_cache_path(name),
            current_version.to_string(),
            "https://github.com/obsidian-full-calendar-remastered/FCR-Reminder-Companion-App/"
                .to_string(),
            latest_api_url,
        )
        .unwrap()
    }

    fn spawn_single_response_server(response: String) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 4096];
                let _ = stream.read(&mut buffer);
                let _ = stream.write_all(response.as_bytes());
                let _ = stream.flush();
            }
        });

        format!("http://{}/releases/latest", addr)
    }

    fn spawn_timeout_server(sleep_ms: u64) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                let mut buffer = [0_u8; 1024];
                let _ = stream.read(&mut buffer);
                thread::sleep(Duration::from_millis(sleep_ms));
                let _ = stream.flush();
            }
        });

        format!("http://{}/releases/latest", addr)
    }

    fn release_response(tag: &str, etag: &str) -> String {
        let body = format!(
            r#"{{"tag_name":"{}","name":"Release {}","html_url":"https://github.com/obsidian-full-calendar-remastered/FCR-Reminder-Companion-App/releases/tag/{}","published_at":"2026-05-22T00:00:00Z","draft":false,"prerelease":false}}"#,
            tag, tag, tag
        );

        format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nETag: {}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            etag,
            body.len(),
            body
        )
    }

    #[test]
    fn normalizes_release_tags() {
        assert_eq!(normalize_release_tag("v0.1.1").unwrap(), "0.1.1");
        assert_eq!(normalize_release_tag("0.1.1").unwrap(), "0.1.1");
    }

    #[test]
    fn compares_versions() {
        let current = Version::parse("0.1.1").unwrap();
        assert!(is_newer_version("0.1.2", &current));
        assert!(!is_newer_version("0.1.1", &current));
        assert!(!is_newer_version("0.1.0", &current));
    }

    #[test]
    fn builds_latest_release_api_url() {
        let api_url = build_latest_api_url(
            "https://github.com/obsidian-full-calendar-remastered/FCR-Reminder-Companion-App/"
        )
        .unwrap();

        assert_eq!(
            api_url,
            "https://api.github.com/repos/obsidian-full-calendar-remastered/FCR-Reminder-Companion-App/releases/latest"
        );
    }

    #[test]
    fn network_retry_delay_starts_short_then_backs_off() {
        assert_eq!(network_retry_delay_seconds(0), INITIAL_NETWORK_RETRY_SECONDS);
        assert_eq!(network_retry_delay_seconds(1), INITIAL_NETWORK_RETRY_SECONDS);
        assert_eq!(network_retry_delay_seconds(2), FOLLOWUP_NETWORK_RETRY_SECONDS);
        assert_eq!(network_retry_delay_seconds(5), FOLLOWUP_NETWORK_RETRY_SECONDS);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn refresh_detects_new_release_and_marks_update_available() {
        let api_url = spawn_single_response_server(release_response("v0.1.2", "\"etag-1\""));
        let service = test_service("new-release", api_url, "0.1.1", 500);

        let refresh = service.refresh_if_due(true).await;

        assert!(refresh.snapshot.update_available);
        assert_eq!(
            refresh.snapshot.latest_release.as_ref().map(|release| release.version.as_str()),
            Some("0.1.2")
        );
        assert!(refresh.should_notify.is_some());

        let _ = std::fs::remove_file(&service.cache_path);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn refresh_network_failure_only_degrades_update_state() {
        let api_url = spawn_timeout_server(300);
        let service = test_service("timeout", api_url, "0.1.1", 100);

        let refresh = service.refresh_if_due(true).await;

        assert!(!refresh.snapshot.update_available);
        assert!(refresh.should_notify.is_none());
        assert!(refresh.snapshot.last_error.is_some());
        assert!(refresh.snapshot.status_label.contains("Update check failed"));
        assert_eq!(
            refresh.snapshot.next_check_at_epoch - refresh.snapshot.last_checked_at_epoch,
            INITIAL_NETWORK_RETRY_SECONDS
        );

        let _ = std::fs::remove_file(&service.cache_path);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn repeated_network_failures_back_off_to_three_days() {
        let api_url = spawn_timeout_server(300);
        let service = test_service("timeout-backoff", api_url, "0.1.1", 100);
        let now = Utc::now().timestamp();
        let cache = UpdateCheckCache {
            last_checked_at_epoch: now - 10,
            next_check_at_epoch: now - 1,
            etag: None,
            latest_release: None,
            update_available: false,
            last_notified_version: None,
            last_error: Some("Previous network failure".to_string()),
            consecutive_network_failures: 1,
        };
        service.save_cache(&cache).unwrap();

        let refresh = service.refresh_if_due(true).await;

        assert_eq!(
            refresh.snapshot.next_check_at_epoch - refresh.snapshot.last_checked_at_epoch,
            FOLLOWUP_NETWORK_RETRY_SECONDS
        );

        let _ = std::fs::remove_file(&service.cache_path);
    }

    #[tokio::test(flavor = "current_thread")]
    async fn refresh_skips_network_when_cache_is_still_fresh() {
        let request_count = Arc::new(AtomicUsize::new(0));
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let request_count_clone = Arc::clone(&request_count);

        thread::spawn(move || {
            if let Ok((mut stream, _)) = listener.accept() {
                request_count_clone.fetch_add(1, Ordering::SeqCst);
                let mut buffer = [0_u8; 1024];
                let _ = stream.read(&mut buffer);
                let _ = stream.write_all(release_response("v0.1.3", "\"etag-2\"").as_bytes());
                let _ = stream.flush();
            }
        });

        let service = test_service(
            "fresh-cache",
            format!("http://{}/releases/latest", addr),
            "0.1.1",
            500,
        );
        let now = Utc::now().timestamp();
        let cache = UpdateCheckCache {
            last_checked_at_epoch: now,
            next_check_at_epoch: now + 3600,
            etag: None,
            latest_release: Some(ReleaseInfo {
                version: "0.1.1".to_string(),
                name: "Release v0.1.1".to_string(),
                html_url: "https://example.test/release".to_string(),
                published_at_rfc3339: "2026-05-22T00:00:00Z".to_string(),
                published_at_epoch: now,
            }),
            update_available: false,
            last_notified_version: None,
            last_error: None,
            consecutive_network_failures: 0,
        };
        service.save_cache(&cache).unwrap();

        let refresh = service.refresh_if_due(false).await;

        assert_eq!(request_count.load(Ordering::SeqCst), 0);
        assert!(!refresh.snapshot.update_available);

        let _ = std::fs::remove_file(&service.cache_path);
    }

    #[test]
    fn action_url_prefers_release_url_when_present() {
        let snapshot = UpdateStateSnapshot {
            current_version: "0.1.1".to_string(),
            update_available: true,
            latest_release: Some(ReleaseInfo {
                version: "0.1.2".to_string(),
                name: "Release v0.1.2".to_string(),
                html_url: "https://example.test/releases/tag/v0.1.2".to_string(),
                published_at_rfc3339: "2026-05-22T00:00:00Z".to_string(),
                published_at_epoch: 0,
            }),
            last_checked_at_epoch: 0,
            next_check_at_epoch: 0,
            last_notified_version: None,
            last_error: None,
            releases_page_url: "https://example.test/releases/latest".to_string(),
            status_label: String::new(),
            menu_label: String::new(),
        };

        assert_eq!(snapshot.action_url(), "https://example.test/releases/tag/v0.1.2");
    }
}