//! Downloading, with a size limit and timeouts.

use std::io::Read;
use std::time::Duration;

use anyhow::{Context, Result, bail};
use url::Url;

/// Longest a connection or a whole download may take.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const TOTAL_TIMEOUT: Duration = Duration::from_secs(300);

/// Largest catalog and archive accepted.
pub const MAX_CATALOG_BYTES: u64 = 5 << 20;
pub const MAX_ARCHIVE_BYTES: u64 = 512 << 20;

/// The body of `url`, refusing more than `max` bytes.
pub fn get(url: &Url, max: u64) -> Result<Vec<u8>> {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(CONNECT_TIMEOUT)
        .timeout(TOTAL_TIMEOUT)
        .build();
    let response = agent
        .get(url.as_str())
        .call()
        .with_context(|| format!("cannot download {url}"))?;
    let mut body = Vec::new();
    response
        .into_reader()
        .take(max + 1)
        .read_to_end(&mut body)?;
    if body.len() as u64 > max {
        bail!("{url} is larger than {max} bytes");
    }
    Ok(body)
}
