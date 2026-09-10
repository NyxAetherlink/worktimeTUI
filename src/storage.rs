use crate::model::Data;
use fs2::FileExt;
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

pub struct Store {
    pub path: PathBuf,
    _lock: File,
}
impl Store {
    pub fn open(dir: &Path) -> io::Result<(Self, Data)> {
        fs::create_dir_all(dir)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(dir.join("instance.lock"))?;
        lock.try_lock_exclusive().map_err(|_| {
            io::Error::other("Another worktimeTUI instance is using this data directory")
        })?;
        let path = dir.join("projects.json");
        let data: Data = match fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Data::default(),
            Err(e) => return Err(e),
        };
        if data.version != 1 {
            return Err(io::Error::other(
                "Unsupported data version; file was not changed",
            ));
        }
        Ok((Self { path, _lock: lock }, data))
    }
    pub fn save(&self, data: &Data) -> io::Result<()> {
        let tmp = self.path.with_extension("json.tmp");
        let mut file = File::create(&tmp)?;
        file.write_all(&serde_json::to_vec_pretty(data).map_err(io::Error::other)?)?;
        file.sync_all()?;
        fs::rename(tmp, &self.path)?;
        File::open(self.path.parent().unwrap())?.sync_all()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Project;
    #[test]
    fn persistence_and_lock() {
        let dir = tempfile::tempdir().unwrap();
        let (store, mut data) = Store::open(dir.path()).unwrap();
        data.projects.push(Project {
            name: "Test".into(),
            focus_ms: 1245,
            break_ms: 120,
        });
        store.save(&data).unwrap();
        assert!(Store::open(dir.path()).is_err());
        drop(store);
        let (_, loaded) = Store::open(dir.path()).unwrap();
        assert_eq!(loaded.projects[0].focus_ms, 1245);
        assert_eq!(loaded.projects[0].break_ms, 120);
    }
    #[test]
    fn malformed_data_is_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("projects.json");
        fs::write(&path, "bad json").unwrap();
        assert!(Store::open(dir.path()).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "bad json");
    }
}
