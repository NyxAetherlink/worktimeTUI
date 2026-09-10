use crate::model::Data;
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Refresh the app-owned spreadsheet snapshot, without appending duplicate rows.
pub fn export(data: &Data, data_dir: &Path) -> io::Result<PathBuf> {
    let dir = data_dir.join("exports");
    fs::create_dir_all(&dir)?;
    let path = dir.join("project-totals.csv");
    let tmp = dir.join("project-totals.csv.tmp");
    let mut file = File::create(&tmp)?;
    // UTF-8 BOM helps Excel recognize Unicode project names.
    file.write_all(b"\xef\xbb\xbfProject,Focus seconds,Break seconds,Tracked seconds,Tracked hours,Breaks included\r\n")?;
    for p in &data.projects {
        let tracked = p.focus_ms + if data.include_breaks { p.break_ms } else { 0 };
        writeln!(
            file,
            "{},{:.3},{:.3},{:.3},{:.6},{}\r",
            text_cell(&p.name),
            p.focus_ms as f64 / 1000.0,
            p.break_ms as f64 / 1000.0,
            tracked as f64 / 1000.0,
            tracked as f64 / 3_600_000.0,
            data.include_breaks
        )?;
    }
    file.sync_all()?;
    fs::rename(tmp, &path)?;
    File::open(&dir)?.sync_all()?;
    Ok(path)
}

fn text_cell(name: &str) -> String {
    // Quoting alone does not prevent spreadsheet formula execution.
    let suspicious =
        name.trim_start().starts_with(['=', '+', '-', '@']) || name.starts_with(['\t', '\r', '\n']);
    format!(
        "\"{}{}\"",
        if suspicious { "'" } else { "" },
        name.replace('"', "\"\"")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Project;
    #[test]
    fn export_refreshes_totals_and_break_policy() {
        let dir = tempfile::tempdir().unwrap();
        let mut data = Data::default();
        data.projects.push(Project {
            name: "Client, \"北\"".into(),
            focus_ms: 3_600_000,
            break_ms: 60_000,
        });
        let path = export(&data, dir.path()).unwrap();
        let first = fs::read_to_string(&path).unwrap();
        assert!(first.contains("\"Client, \"\"北\"\"\",3600.000,60.000,3600.000,1.000000,false"));
        data.include_breaks = true;
        data.projects[0].focus_ms += 500;
        assert_eq!(export(&data, dir.path()).unwrap(), path);
        let updated = fs::read_to_string(path).unwrap();
        assert!(updated.contains(",3600.500,60.000,3660.500,1.016806,true"));
        assert_eq!(updated.lines().count(), 2);
    }
    #[test]
    fn formula_like_names_are_text() {
        for name in ["=SUM(A1)", " +1", "-1", "@test", "\tformula"] {
            assert!(text_cell(name).starts_with("\"'"));
        }
        assert_eq!(text_cell("Normal"), "\"Normal\"");
    }
    #[test]
    fn empty_export_has_headers() {
        let dir = tempfile::tempdir().unwrap();
        let path = export(&Data::default(), dir.path()).unwrap();
        assert_eq!(fs::read_to_string(path).unwrap().lines().count(), 1);
    }
}
