use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::db::{Database, PageRecord};
use crate::error::LinkdropError;
use crate::id::random_id;
use crate::slug::validate_slug;

pub const MAX_HTML_BYTES: usize = 5 * 1024 * 1024;

pub struct Storage {
    data_dir: PathBuf,
    html_dir: PathBuf,
    db: Database,
}

impl Storage {
    pub fn open(data_dir: PathBuf) -> anyhow::Result<Self> {
        let html_dir = data_dir.join("html");
        std::fs::create_dir_all(&html_dir)?;
        let db = Database::open(&data_dir.join("linkdrop.db"))?;
        Ok(Self {
            data_dir,
            html_dir,
            db,
        })
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn put(
        &self,
        slug: Option<&str>,
        content: &[u8],
        force: bool,
        ttl: Option<Duration>,
    ) -> Result<PageRecord, LinkdropError> {
        if content.len() > MAX_HTML_BYTES {
            return Err(LinkdropError::TooLarge {
                max: MAX_HTML_BYTES,
            });
        }

        let slug = match slug {
            Some(s) => {
                validate_slug(s)?;
                s.to_string()
            }
            None => random_id(),
        };

        let existing = self.db.get(&slug)?;
        if existing.is_some() && !force {
            return Err(LinkdropError::SlugExists(slug));
        }

        let filename = format!("{slug}.html");
        let path = self.html_dir.join(&filename);
        std::fs::write(&path, content).map_err(|e| LinkdropError::Other(e.into()))?;

        let record = if existing.is_some() {
            self.db.update(&slug, &filename, content.len(), ttl)?
        } else {
            self.db.insert(&slug, &filename, content.len(), ttl)?
        };

        Ok(record)
    }

    pub fn get(&self, slug: &str) -> Result<(PageRecord, Vec<u8>), LinkdropError> {
        validate_slug(slug)?;

        let record = self
            .db
            .get(slug)?
            .ok_or_else(|| LinkdropError::NotFound(slug.to_string()))?;

        Database::check_not_expired(&record)?;

        let path = self.html_dir.join(&record.filename);
        let content =
            std::fs::read(&path).map_err(|_| LinkdropError::NotFound(slug.to_string()))?;

        Ok((record, content))
    }

    pub fn delete(&self, slug: &str) -> Result<bool, LinkdropError> {
        validate_slug(slug)?;

        let record = self
            .db
            .get(slug)?
            .ok_or_else(|| LinkdropError::NotFound(slug.to_string()))?;

        let path = self.html_dir.join(&record.filename);
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| LinkdropError::Other(e.into()))?;
        }

        Ok(self.db.delete(slug)?)
    }

    pub fn list(&self) -> anyhow::Result<Vec<PageRecord>> {
        self.db.list()
    }

    pub fn sweep_expired(&self) -> anyhow::Result<usize> {
        let slugs = self.db.expired_slugs()?;
        let mut removed = 0;

        for slug in slugs {
            if self.delete(&slug).unwrap_or(false) {
                removed += 1;
            }
        }

        Ok(removed)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn storage_should_keep_pages_after_reopen() {
        let data_dir = unique_temp_dir();
        let html = b"<html><body>persist me</body></html>";

        {
            let storage = Storage::open(data_dir.clone()).unwrap();
            storage.put(Some("persist-me"), html, false, None).unwrap();
        }

        let storage = Storage::open(data_dir.clone()).unwrap();
        let (record, content) = storage.get("persist-me").unwrap();

        assert_eq!(record.slug, "persist-me");
        assert_eq!(content, html);

        std::fs::remove_dir_all(data_dir).unwrap();
    }

    fn unique_temp_dir() -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("linkdrop-storage-test-{nanos}"))
    }
}
