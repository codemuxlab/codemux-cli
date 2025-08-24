use anyhow::Result;
use portable_pty::{CommandBuilder, NativePtySystem, PtySize, PtySystem};
use std::io::{Read, Write};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct PtySession {
    pub _id: String,
    pub agent: String,
    pub _args: Vec<String>,
    pub pty: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
    pub reader: Arc<Mutex<Box<dyn std::io::Read + Send>>>,
    pub writer: Arc<Mutex<Box<dyn std::io::Write + Send>>>,
}

impl PtySession {
    pub fn new(id: String, agent: String, args: Vec<String>) -> Result<Self> {
        let pty_system = NativePtySystem::default();
        
        let pty_pair = pty_system.openpty(PtySize {
            rows: 30,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        
        let mut cmd = CommandBuilder::new(&agent);
        for arg in &args {
            cmd.arg(arg);
        }
        
        // Set working directory to current directory (project root)
        if let Ok(current_dir) = std::env::current_dir() {
            cmd.cwd(current_dir);
        }
        
        // Set environment variables for proper terminal behavior
        cmd.env("TERM", "xterm-256color"); // Proper terminal for full functionality
        cmd.env("COLORTERM", "truecolor");
        cmd.env("FORCE_COLOR", "1");
        cmd.env("COLUMNS", "120");
        cmd.env("LINES", "30");
        
        // Preserve all important environment variables from current session
        for (key, value) in std::env::vars() {
            match key.as_str() {
                "HOME" | "USER" | "PATH" | "SHELL" | "LANG" | "LC_ALL" | "PWD" => {
                    cmd.env(key, value);
                }
                _ => {}
            }
        }
        
        let _child = pty_pair.slave.spawn_command(cmd)?;
        
        let reader = pty_pair.master.try_clone_reader()?;
        let writer = pty_pair.master.take_writer()?;
        
        Ok(PtySession {
            _id: id,
            agent,
            _args: args,
            pty: Arc::new(Mutex::new(pty_pair.master)),
            reader: Arc::new(Mutex::new(reader)),
            writer: Arc::new(Mutex::new(writer)),
        })
    }
    
    pub async fn _write(&self, data: &[u8]) -> Result<()> {
        let mut writer = self.writer.lock().await;
        writer.write_all(data)?;
        writer.flush()?;
        Ok(())
    }
    
    pub async fn _read(&self, buf: &mut [u8]) -> Result<usize> {
        let mut reader = self.reader.lock().await;
        let n = reader.read(buf)?;
        Ok(n)
    }
    
    pub async fn _resize(&self, rows: u16, cols: u16) -> Result<()> {
        let pty = self.pty.lock().await;
        pty.resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }
}