use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct StoredCommand {
    pub cmd: String,
    #[serde(default)]
    pub count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandEntry {
    pub cmd: String,
    pub count: usize,
    pub is_global: bool,
    pub original_index: usize, // Index in the respective local/global vector
}

pub struct Store {
    pub local_path: Option<PathBuf>,
    pub global_path: Option<PathBuf>,
    pub local_cmds: Vec<StoredCommand>,
    pub global_cmds: Vec<StoredCommand>,
}

impl Store {
    pub fn load() -> Self {
        let local_path = PathBuf::from(".cbox");
        let local_path = if local_path.exists()
            || fs::metadata(".")
                .map(|m| !m.permissions().readonly())
                .unwrap_or(false)
        {
            Some(local_path)
        } else {
            None
        };

        let mut global_path = dirs::config_dir().map(|p| p.join("cbox").join("commands.json"));
        // fallback if no config dir
        if global_path.is_none()
            && let Some(home) = dirs::home_dir()
        {
            global_path = Some(home.join(".cbox_global.json"));
        }

        let mut local_cmds = Vec::new();
        if let Some(p) = &local_path {
            local_cmds = load_file(p);
            // check legacy .rast
            let legacy = PathBuf::from(".rast");
            if local_cmds.is_empty() && legacy.exists() {
                local_cmds = load_legacy(&legacy);
                // attempt to rename
                let _ = fs::rename(legacy, p);
            }
        }

        let mut global_cmds = Vec::new();
        if let Some(p) = &global_path {
            global_cmds = load_file(p);
        }

        Self {
            local_path,
            global_path,
            local_cmds,
            global_cmds,
        }
    }

    pub fn save_local(&self) -> io::Result<()> {
        if let Some(p) = &self.local_path {
            save_file(p, &self.local_cmds)?;
        }
        Ok(())
    }

    pub fn save_global(&self) -> io::Result<()> {
        if let Some(p) = &self.global_path {
            if let Some(parent) = p.parent() {
                let _ = fs::create_dir_all(parent);
            }
            save_file(p, &self.global_cmds)?;
        }
        Ok(())
    }

    pub fn get_all(&self) -> Vec<CommandEntry> {
        let mut entries = Vec::new();
        for (i, c) in self.local_cmds.iter().enumerate() {
            entries.push(CommandEntry {
                cmd: c.cmd.clone(),
                count: c.count,
                is_global: false,
                original_index: i,
            });
        }
        for (i, c) in self.global_cmds.iter().enumerate() {
            // deduplicate if same command exists in local
            if !self.local_cmds.iter().any(|lc| lc.cmd == c.cmd) {
                entries.push(CommandEntry {
                    cmd: c.cmd.clone(),
                    count: c.count,
                    is_global: true,
                    original_index: i,
                });
            }
        }
        entries
    }

    pub fn increment_usage(&mut self, is_global: bool, original_index: usize) -> io::Result<()> {
        if is_global {
            if original_index < self.global_cmds.len() {
                self.global_cmds[original_index].count += 1;
                self.save_global()?;
            }
        } else {
            if original_index < self.local_cmds.len() {
                self.local_cmds[original_index].count += 1;
                self.save_local()?;
            }
        }
        Ok(())
    }

    pub fn add_command(&mut self, cmd: &str, global: bool) -> io::Result<()> {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            return Ok(());
        }
        if global {
            if let Some(pos) = self.global_cmds.iter().position(|c| c.cmd == cmd) {
                let existing = self.global_cmds.remove(pos);
                self.global_cmds.push(existing);
            } else {
                self.global_cmds.push(StoredCommand {
                    cmd: cmd.to_string(),
                    count: 0,
                });
            }
            self.save_global()?;
        } else {
            if let Some(pos) = self.local_cmds.iter().position(|c| c.cmd == cmd) {
                let existing = self.local_cmds.remove(pos);
                self.local_cmds.push(existing);
            } else {
                self.local_cmds.push(StoredCommand {
                    cmd: cmd.to_string(),
                    count: 0,
                });
            }
            self.save_local()?;
        }
        Ok(())
    }

    pub fn update_command(
        &mut self,
        is_global: bool,
        original_index: usize,
        new_text: String,
    ) -> io::Result<()> {
        if is_global {
            if original_index < self.global_cmds.len() {
                self.global_cmds[original_index].cmd = new_text;
                self.save_global()?;
            }
        } else {
            if original_index < self.local_cmds.len() {
                self.local_cmds[original_index].cmd = new_text;
                self.save_local()?;
            }
        }
        Ok(())
    }

    pub fn delete_command(&mut self, is_global: bool, original_index: usize) -> io::Result<String> {
        let removed = if is_global {
            if original_index < self.global_cmds.len() {
                let r = self.global_cmds.remove(original_index);
                self.save_global()?;
                r.cmd
            } else {
                String::new()
            }
        } else {
            if original_index < self.local_cmds.len() {
                let r = self.local_cmds.remove(original_index);
                self.save_local()?;
                r.cmd
            } else {
                String::new()
            }
        };
        Ok(removed)
    }
}

fn load_file(path: &PathBuf) -> Vec<StoredCommand> {
    match fs::read_to_string(path) {
        Ok(s) => {
            if let Ok(cmds) = serde_json::from_str::<Vec<StoredCommand>>(&s) {
                cmds
            } else {
                // Try legacy text fallback
                load_legacy_str(&s)
            }
        }
        Err(_) => Vec::new(),
    }
}

fn load_legacy(path: &PathBuf) -> Vec<StoredCommand> {
    if let Ok(s) = fs::read_to_string(path) {
        load_legacy_str(&s)
    } else {
        Vec::new()
    }
}

fn load_legacy_str(s: &str) -> Vec<StoredCommand> {
    s.lines()
        .map(|l| l.trim_end_matches('\r').to_string())
        .filter(|l| !l.trim().is_empty())
        .map(|cmd| StoredCommand { cmd, count: 0 })
        .collect()
}

fn save_file(path: &PathBuf, cmds: &[StoredCommand]) -> io::Result<()> {
    let body = serde_json::to_string_pretty(cmds)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    fs::write(path, body)
}
