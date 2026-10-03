use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandEntry {
    pub cmd: String,
    pub is_global: bool,
    pub original_index: usize, // Index in the respective local/global vector
}

pub struct Store {
    pub local_path: Option<PathBuf>,
    pub global_path: Option<PathBuf>,
    pub local_cmds: Vec<String>,
    pub global_cmds: Vec<String>,
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

        let mut global_path = dirs::config_dir().map(|p| p.join("cbox").join("commands"));
        if global_path.is_none()
            && let Some(home) = dirs::home_dir()
        {
            global_path = Some(home.join(".cbox_global"));
        }

        let mut local_cmds = Vec::new();
        if let Some(p) = &local_path {
            local_cmds = load_file(p);
            let legacy = PathBuf::from(".rast");
            if local_cmds.is_empty() && legacy.exists() {
                local_cmds = load_file(&legacy);
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
        for (i, cmd) in self.local_cmds.iter().enumerate() {
            entries.push(CommandEntry {
                cmd: cmd.clone(),
                is_global: false,
                original_index: i,
            });
        }
        for (i, cmd) in self.global_cmds.iter().enumerate() {
            if !self.local_cmds.iter().any(|lc| lc == cmd) {
                entries.push(CommandEntry {
                    cmd: cmd.clone(),
                    is_global: true,
                    original_index: i,
                });
            }
        }
        entries
    }

    pub fn add_command(&mut self, cmd: &str, global: bool) -> io::Result<()> {
        let cmd = cmd.trim();
        if cmd.is_empty() {
            return Ok(());
        }
        if global {
            if let Some(pos) = self.global_cmds.iter().position(|c| c == cmd) {
                let existing = self.global_cmds.remove(pos);
                self.global_cmds.push(existing);
            } else {
                self.global_cmds.push(cmd.to_string());
            }
            self.save_global()?;
        } else {
            if let Some(pos) = self.local_cmds.iter().position(|c| c == cmd) {
                let existing = self.local_cmds.remove(pos);
                self.local_cmds.push(existing);
            } else {
                self.local_cmds.push(cmd.to_string());
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
                self.global_cmds[original_index] = new_text;
                self.save_global()?;
            }
        } else {
            if original_index < self.local_cmds.len() {
                self.local_cmds[original_index] = new_text;
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
                r
            } else {
                String::new()
            }
        } else {
            if original_index < self.local_cmds.len() {
                let r = self.local_cmds.remove(original_index);
                self.save_local()?;
                r
            } else {
                String::new()
            }
        };
        Ok(removed)
    }
}

fn load_file(path: &PathBuf) -> Vec<String> {
    match fs::read_to_string(path) {
        Ok(s) => s
            .lines()
            .map(|l| l.trim_end_matches('\r').to_string())
            .filter(|l| !l.trim().is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

fn save_file(path: &PathBuf, cmds: &[String]) -> io::Result<()> {
    let body = if cmds.is_empty() {
        String::new()
    } else {
        let mut s = cmds.join("\n");
        s.push('\n');
        s
    };
    fs::write(path, body)
}
