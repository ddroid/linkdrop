use regex::Regex;
use std::sync::LazyLock;

use crate::error::LinkdropError;

static SLUG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-z0-9](?:[a-z0-9-]*[a-z0-9])?$").unwrap());

pub const MAX_SLUG_LEN: usize = 64;

pub fn validate_slug(slug: &str) -> Result<(), LinkdropError> {
    if slug.is_empty() || slug.len() > MAX_SLUG_LEN {
        return Err(LinkdropError::InvalidSlug(
            "slug must be 1-64 characters".into(),
        ));
    }

    if !SLUG_RE.is_match(slug) {
        return Err(LinkdropError::InvalidSlug(
            "slug may only contain lowercase letters, digits, and hyphens (no leading/trailing hyphen)"
                .into(),
        ));
    }

    Ok(())
}
