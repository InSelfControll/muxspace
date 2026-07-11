use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static SNAPSHOT_ID: AtomicU64 = AtomicU64::new(0);

pub struct SqliteSnapshot {
    dir: PathBuf,
}

impl SqliteSnapshot {
    pub fn open(source: &Path) -> Result<(Connection, Self)> {
        let file_name = source
            .file_name()
            .context("SQLite source has no file name")?;
        let id = SNAPSHOT_ID.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("muxspace-sqlite-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&dir)?;
        let snapshot = Self { dir };
        let target = snapshot.dir.join(file_name);
        std::fs::copy(source, &target)
            .with_context(|| format!("Copying SQLite database {}", source.display()))?;
        snapshot.copy_sidecar(source, file_name, "-wal")?;
        snapshot.copy_sidecar(source, file_name, "-shm")?;

        let connection = Connection::open_with_flags(
            target,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        Ok((connection, snapshot))
    }

    fn copy_sidecar(&self, source: &Path, file_name: &std::ffi::OsStr, suffix: &str) -> Result<()> {
        let mut source_name = OsString::from(source.as_os_str());
        source_name.push(suffix);
        let source_sidecar = PathBuf::from(source_name);
        if source_sidecar.exists() {
            let mut target_name = OsString::from(file_name);
            target_name.push(suffix);
            std::fs::copy(&source_sidecar, self.dir.join(target_name))
                .with_context(|| format!("Copying SQLite sidecar {}", source_sidecar.display()))?;
        }
        Ok(())
    }
}

impl Drop for SqliteSnapshot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snapshot_reads_wal_while_source_is_open() {
        let source_dir =
            std::env::temp_dir().join(format!("muxspace-snapshot-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&source_dir);
        std::fs::create_dir_all(&source_dir).unwrap();
        let source = source_dir.join("cookies.sqlite");
        let connection = Connection::open(&source).unwrap();
        connection
            .pragma_update(None, "journal_mode", "WAL")
            .unwrap();
        connection
            .pragma_update(None, "wal_autocheckpoint", 0)
            .unwrap();
        connection
            .execute("CREATE TABLE cookies (value TEXT NOT NULL)", [])
            .unwrap();
        connection
            .execute("INSERT INTO cookies VALUES ('authenticated')", [])
            .unwrap();

        let (snapshot, _guard) = SqliteSnapshot::open(&source).unwrap();
        let value: String = snapshot
            .query_row("SELECT value FROM cookies", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "authenticated");

        drop(snapshot);
        drop(connection);
        std::fs::remove_dir_all(source_dir).unwrap();
    }
}
