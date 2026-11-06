//! Ghi log ra file %APPDATA%\JokerTool\log.txt

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

pub struct LogWriter {
    file: Option<File>,
    path: PathBuf,
}

impl LogWriter {
    pub fn new(path: PathBuf) -> Self {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .ok();
        let mut w = Self { file, path };
        if w.file.is_some() {
            w.write_raw(&format!(
                "\n========== Phiên mới {} ==========\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            ));
        }
        w
    }

    fn write_raw(&mut self, line: &str) {
        if let Some(f) = &mut self.file {
            let _ = writeln!(f, "{}", line);
            let _ = f.flush();
        }
    }

    pub fn write(&mut self, line: &str) {
        let ts = chrono::Local::now().format("%H:%M:%S");
        self.write_raw(&format!("[{}] {}", ts, line));
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn open_folder(&self) {
        if let Some(parent) = self.path.parent() {
            #[cfg(windows)]
            {
                let _ = std::process::Command::new("explorer").arg(parent).spawn();
            }
            #[cfg(not(windows))]
            {
                let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
            }
        }
    }
}
