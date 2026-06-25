use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

use crate::error::LinkdropError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageRecord {
    pub slug: String,
    pub filename: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub size_bytes: i64,
}

pub struct Database {
    conn: Mutex<Connection>,
}

impl Database {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(path)?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS pages (
                slug TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER,
                size_bytes INTEGER NOT NULL
            );",
        )?;

        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    pub fn insert(
        &self,
        slug: &str,
        filename: &str,
        size_bytes: usize,
        ttl: Option<Duration>,
    ) -> anyhow::Result<PageRecord> {
        let created_at = Utc::now();
        let expires_at = ttl.map(|d| created_at + chrono::Duration::from_std(d).unwrap_or_default());
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO pages (slug, filename, created_at, expires_at, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                slug,
                filename,
                created_at.timestamp(),
                expires_at.map(|t| t.timestamp()),
                size_bytes as i64,
            ],
        )?;

        Ok(PageRecord {
            slug: slug.to_string(),
            filename: filename.to_string(),
            created_at,
            expires_at,
            size_bytes: size_bytes as i64,
        })
    }

    pub fn update(
        &self,
        slug: &str,
        filename: &str,
        size_bytes: usize,
        ttl: Option<Duration>,
    ) -> anyhow::Result<PageRecord> {
        let created_at = Utc::now();
        let expires_at = ttl.map(|d| created_at + chrono::Duration::from_std(d).unwrap_or_default());
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "UPDATE pages
             SET filename = ?2, created_at = ?3, expires_at = ?4, size_bytes = ?5
             WHERE slug = ?1",
            params![
                slug,
                filename,
                created_at.timestamp(),
                expires_at.map(|t| t.timestamp()),
                size_bytes as i64,
            ],
        )?;

        Ok(PageRecord {
            slug: slug.to_string(),
            filename: filename.to_string(),
            created_at,
            expires_at,
            size_bytes: size_bytes as i64,
        })
    }

    pub fn get(&self, slug: &str) -> anyhow::Result<Option<PageRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT slug, filename, created_at, expires_at, size_bytes FROM pages WHERE slug = ?1",
        )?;

        let mut rows = stmt.query(params![slug])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(row_to_record(row)?));
        }

        Ok(None)
    }

    pub fn list(&self) -> anyhow::Result<Vec<PageRecord>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT slug, filename, created_at, expires_at, size_bytes FROM pages ORDER BY created_at DESC",
        )?;

        let rows = stmt.query_map([], |row| {
            Ok(PageRecord {
                slug: row.get(0)?,
                filename: row.get(1)?,
                created_at: DateTime::from_timestamp(row.get::<_, i64>(2)?, 0)
                    .unwrap_or_default()
                    .with_timezone(&Utc),
                expires_at: row
                    .get::<_, Option<i64>>(3)?
                    .and_then(|ts| DateTime::from_timestamp(ts, 0))
                    .map(|dt| dt.with_timezone(&Utc)),
                size_bytes: row.get(4)?,
            })
        })?;

        let mut pages = Vec::new();
        for row in rows {
            pages.push(row?);
        }
        Ok(pages)
    }

    pub fn delete(&self, slug: &str) -> anyhow::Result<bool> {
        let conn = self.conn.lock().unwrap();
        let changed = conn.execute("DELETE FROM pages WHERE slug = ?1", params![slug])?;
        Ok(changed > 0)
    }

    pub fn expired_slugs(&self) -> anyhow::Result<Vec<String>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let conn = self.conn.lock().unwrap();
        let mut stmt = conn
            .prepare("SELECT slug FROM pages WHERE expires_at IS NOT NULL AND expires_at <= ?1")?;

        let rows = stmt.query_map(params![now], |row| row.get(0))?;
        let mut slugs = Vec::new();
        for slug in rows {
            slugs.push(slug?);
        }
        Ok(slugs)
    }

    pub fn is_expired(record: &PageRecord) -> bool {
        match record.expires_at {
            Some(expires_at) => expires_at <= Utc::now(),
            None => false,
        }
    }

    pub fn check_not_expired(record: &PageRecord) -> Result<(), LinkdropError> {
        if Self::is_expired(record) {
            return Err(LinkdropError::Expired(record.slug.clone()));
        }
        Ok(())
    }
}

fn row_to_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<PageRecord> {
    Ok(PageRecord {
        slug: row.get(0)?,
        filename: row.get(1)?,
        created_at: DateTime::from_timestamp(row.get::<_, i64>(2)?, 0)
            .unwrap_or_default()
            .with_timezone(&Utc),
        expires_at: row
            .get::<_, Option<i64>>(3)?
            .and_then(|ts| DateTime::from_timestamp(ts, 0))
            .map(|dt| dt.with_timezone(&Utc)),
        size_bytes: row.get(4)?,
    })
}
