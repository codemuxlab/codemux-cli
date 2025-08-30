use anyhow::Result;
use chrono::{DateTime, Utc};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

/// Convert Claude's encoded project path to actual filesystem path
/// e.g., "-Users-cinoss-Code-playground-mojo" -> Some("/Users/cinoss/Code/playground/mojo")  
/// e.g., "-a-b.c-d" -> Some("/a/b.c/d") (using filesystem search with glob patterns)
/// Returns None if no valid filesystem path can be constructed
/// Uses left-to-right filesystem search to handle any directory names
fn decode_claude_project_path(encoded: &str) -> Option<PathBuf> {
    // Remove leading hyphen if present
    let path_part = if let Some(stripped) = encoded.strip_prefix('-') {
        stripped
    } else {
        encoded
    };
    
    if path_part.is_empty() {
        return None;
    }
    
    let segments: Vec<&str> = path_part.split('-').collect();
    if segments.is_empty() {
        return None;
    }
    
    let mut current_path = PathBuf::from("/");
    let mut remaining_segments = segments.as_slice();
    
    // Build path left-to-right using filesystem search
    while !remaining_segments.is_empty() {
        let target_segment = remaining_segments[0];
        let mut found_match = false;
        
        // Search for directories that start with the target segment
        if let Ok(entries) = std::fs::read_dir(&current_path) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    // Check if this directory name matches our target segment
                    if name.starts_with(target_segment) {
                        // Split the found name by non-alphanumeric chars and check if it matches
                        let name_parts: Vec<&str> = name.split(|c: char| !c.is_alphanumeric()).collect();
                        let name_alpha_parts: Vec<&str> = name_parts.into_iter().filter(|s| !s.is_empty()).collect();
                        
                        // Check if the alphanumeric parts exactly match the beginning of remaining segments
                        if name_alpha_parts.len() <= remaining_segments.len() &&
                           name_alpha_parts == remaining_segments[..name_alpha_parts.len()] &&
                           entry.path().is_dir() {
                            // Found an exact match, move to this directory
                            current_path = entry.path();
                            remaining_segments = &remaining_segments[name_alpha_parts.len()..];
                            found_match = true;
                            break;
                        }
                    }
                }
            }
        }
        
        // If no match found, we can't proceed further
        if !found_match {
            return None;
        }
    }
    
    // Return the path only if we consumed all segments and it exists
    if current_path.exists() && current_path != PathBuf::from("/") {
        Some(current_path)
    } else {
        None
    }
}

/// Convert filesystem path to Claude's encoded format
/// e.g., "/Users/cinoss/Code/playground/mojo" -> "-Users-cinoss-Code-playground-mojo"
#[allow(dead_code)]
fn encode_claude_project_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    
    // Remove leading slash and replace all slashes with hyphens
    if let Some(stripped) = path_str.strip_prefix('/') {
        format!("-{}", stripped.replace('/', "-"))
    } else {
        path_str.replace('/', "-")
    }
}

/// Represents a cached JSONL session file from .claude/projects
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedSession {
    pub session_id: String,
    pub file_path: PathBuf,
    pub project_path: PathBuf,
    pub agent: String,
    pub last_modified: DateTime<Utc>,
    pub file_size: u64,
    pub is_active: bool,
    pub last_message: Option<String>,
}

/// Events for cache updates
#[derive(Debug, Clone)]
pub enum CacheEvent {
    SessionAdded(CachedSession),
    SessionModified(CachedSession),
    SessionDeleted(String), // session_id
}

/// Cache for .claude projects directory
pub struct ClaudeProjectsCache {
    /// Base path to .claude/projects directory
    base_path: PathBuf,
    /// Cached sessions by session ID
    sessions: Arc<RwLock<HashMap<String, CachedSession>>>,
    /// File system watcher
    watcher: Option<RecommendedWatcher>,
    /// Channel for cache events
    event_tx: mpsc::UnboundedSender<CacheEvent>,
    /// Channel receiver for cache events (for external consumers)
    pub event_rx: Option<mpsc::UnboundedReceiver<CacheEvent>>,
}

impl ClaudeProjectsCache {
    /// Create a new cache for Claude projects
    pub fn new() -> Result<Self> {
        let home_dir = directories::UserDirs::new()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .home_dir()
            .to_path_buf();
        
        let base_path = home_dir.join(".claude").join("projects");
        
        // Create channel for cache events
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        
        Ok(Self {
            base_path,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            watcher: None,
            event_tx,
            event_rx: Some(event_rx),
        })
    }

    /// Initialize the cache by scanning the directory and setting up the watcher
    pub async fn initialize(&mut self) -> Result<()> {
        info!("Initializing Claude projects cache at {:?}", self.base_path);
        
        // Ensure the directory exists
        if !self.base_path.exists() {
            warn!("Claude projects directory does not exist: {:?}", self.base_path);
            tokio::fs::create_dir_all(&self.base_path).await?;
            info!("Created Claude projects directory");
        }
        
        // Initial scan of the directory
        self.scan_directory().await?;
        
        // Set up file system watcher
        self.setup_watcher()?;
        
        Ok(())
    }

    /// Scan the .claude/projects directory for JSONL files
    async fn scan_directory(&self) -> Result<()> {
        info!("Scanning Claude projects directory for JSONL files");
        
        let mut sessions = self.sessions.write().await;
        sessions.clear();
        
        // Walk through all subdirectories looking for .jsonl files
        let mut entries = tokio::fs::read_dir(&self.base_path).await?;
        let mut count = 0;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            
            // Each subdirectory represents a project
            if path.is_dir() {
                // Extract the encoded project name and decode it to actual path
                let encoded_project_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                
                let actual_project_path = match decode_claude_project_path(encoded_project_name) {
                    Some(path) => {
                        debug!("Found Claude project: {} -> {:?}", encoded_project_name, path);
                        path
                    }
                    None => {
                        debug!("Skipping invalid Claude project: {}", encoded_project_name);
                        continue;
                    }
                };
                
                let mut project_entries = tokio::fs::read_dir(&path).await?;
                
                while let Some(file_entry) = project_entries.next_entry().await? {
                    let file_path = file_entry.path();
                    
                    // Look for .jsonl files
                    if file_path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                        if let Ok(session) = self.parse_session_file(&file_path, &actual_project_path).await {
                            sessions.insert(session.session_id.clone(), session.clone());
                            count += 1;
                            debug!("Found session: {} for project {:?}", session.session_id, actual_project_path);
                            
                            // Send event for discovered session
                            let _ = self.event_tx.send(CacheEvent::SessionAdded(session));
                        }
                    }
                }
            }
        }
        
        info!("Found {} JSONL session files", count);
        Ok(())
    }

    /// Parse a JSONL file to extract session information
    async fn parse_session_file(&self, file_path: &Path, project_path: &Path) -> Result<CachedSession> {
        let metadata = tokio::fs::metadata(file_path).await?;
        let last_modified = metadata.modified()?.into();
        let file_size = metadata.len();
        
        // Extract session ID from filename (filename without .jsonl extension)
        let session_id = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?
            .to_string();
        
        // Try to determine the agent from the file content
        // For now, default to "claude" as it's most common
        let agent = self.detect_agent_from_file(file_path).await.unwrap_or_else(|_| "claude".to_string());
        
        // Extract the last message from the file
        let last_message = self.get_last_message_from_file(file_path).await;
        
        // Check if the session is currently active (recently modified)
        let is_active = {
            let now = Utc::now();
            let modified: DateTime<Utc> = last_modified;
            (now - modified).num_minutes() < 5 // Consider active if modified in last 5 minutes
        };
        
        Ok(CachedSession {
            session_id,
            file_path: file_path.to_path_buf(),
            project_path: project_path.to_path_buf(),
            agent,
            last_modified,
            file_size,
            is_active,
            last_message,
        })
    }

    /// Try to detect the agent from the JSONL file content
    async fn detect_agent_from_file(&self, file_path: &Path) -> Result<String> {
        // Read first few lines of the file to detect agent
        let content = tokio::fs::read_to_string(file_path).await?;
        let first_line = content.lines().next().unwrap_or("");
        
        // Parse first line as JSON and look for agent field
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(first_line) {
            if let Some(agent) = json_value.get("agent").and_then(|v| v.as_str()) {
                return Ok(agent.to_string());
            }
        }
        
        // Default to claude if not found
        Ok("claude".to_string())
    }

    /// Extract the 3 most recent messages from a JSONL session file
    async fn get_last_message_from_file(&self, file_path: &Path) -> Option<String> {
        // Read the file content
        let content = match tokio::fs::read_to_string(file_path).await {
            Ok(content) => content,
            Err(_) => return None,
        };
        
        // Get the last 3 non-empty lines and return them as a JSON array string
        let recent_lines: Vec<&str> = content
            .lines()
            .rev()
            .filter(|line| !line.trim().is_empty())
            .take(3)
            .collect::<Vec<_>>()
            .into_iter()
            .rev() // Reverse to get chronological order (oldest to newest)
            .collect();
        
        if recent_lines.is_empty() {
            return None;
        }
        
        // Return as a JSON array string for frontend to parse
        let json_array = format!("[{}]", recent_lines.join(","));
        Some(json_array)
    }

    /// Set up the file system watcher
    fn setup_watcher(&mut self) -> Result<()> {
        let sessions = Arc::clone(&self.sessions);
        let event_tx = self.event_tx.clone();
        let base_path = self.base_path.clone();
        
        // Create a channel for sending events from the watcher to the tokio runtime
        let (fs_event_tx, mut fs_event_rx) = mpsc::unbounded_channel::<Event>();
        
        // Spawn a tokio task to handle file system events
        tokio::spawn(async move {
            while let Some(event) = fs_event_rx.recv().await {
                if let Err(e) = handle_fs_event(event, Arc::clone(&sessions), event_tx.clone(), base_path.clone()).await {
                    error!("Error handling file system event: {}", e);
                }
            }
        });
        
        // Create watcher with debounce config
        let config = Config::default()
            .with_poll_interval(std::time::Duration::from_secs(2));
        
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| {
                match res {
                    Ok(event) => {
                        // Send event to tokio task via channel
                        if let Err(e) = fs_event_tx.send(event) {
                            error!("Failed to send file system event: {}", e);
                        }
                    }
                    Err(e) => error!("File watcher error: {:?}", e),
                }
            },
            config,
        )?;
        
        // Watch the base directory recursively
        watcher.watch(&self.base_path, RecursiveMode::Recursive)?;
        info!("Started watching directory: {:?}", self.base_path);
        
        self.watcher = Some(watcher);
        Ok(())
    }

    /// Get all cached sessions
    pub async fn get_all_sessions(&self) -> Vec<CachedSession> {
        let sessions = self.sessions.read().await;
        sessions.values().cloned().collect()
    }

    /// Get a specific session by ID
    pub async fn get_session(&self, session_id: &str) -> Option<CachedSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Get sessions for a specific project path
    pub async fn get_project_sessions(&self, project_path: &Path) -> Vec<CachedSession> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .filter(|s| s.project_path == project_path)
            .cloned()
            .collect()
    }

    /// Manually refresh the cache
    pub async fn refresh(&self) -> Result<()> {
        self.scan_directory().await
    }
}

/// Handle file system events
async fn handle_fs_event(
    event: Event,
    sessions: Arc<RwLock<HashMap<String, CachedSession>>>,
    event_tx: mpsc::UnboundedSender<CacheEvent>,
    base_path: PathBuf,
) -> Result<()> {
    match event.kind {
        EventKind::Create(_) | EventKind::Modify(_) => {
            // Handle file creation or modification
            for path in event.paths {
                if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                    debug!("JSONL file changed: {:?}", path);
                    
                    // Determine project path by decoding the encoded directory name
                    let encoded_dir = path.parent()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    
                    let project_path = if !encoded_dir.is_empty() && path.parent() != Some(&base_path) {
                        decode_claude_project_path(encoded_dir).unwrap_or_else(|| {
                            debug!("Failed to decode project path: {}, using parent directory", encoded_dir);
                            path.parent().unwrap_or(&base_path).to_path_buf()
                        })
                    } else {
                        path.parent().unwrap_or(&base_path).to_path_buf()
                    };
                    
                    // Parse the session file
                    if let Ok(metadata) = tokio::fs::metadata(&path).await {
                        let last_modified = metadata.modified()?.into();
                        let file_size = metadata.len();
                        
                        let session_id = path
                            .file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("")
                            .to_string();
                        
                        if !session_id.is_empty() {
                            let mut sessions_guard = sessions.write().await;
                            
                            // Check if this is a new or modified session
                            let is_new = !sessions_guard.contains_key(&session_id);
                            
                            let session = CachedSession {
                                session_id: session_id.clone(),
                                file_path: path.clone(),
                                project_path,
                                agent: "claude".to_string(), // Default, could be detected
                                last_modified,
                                file_size,
                                is_active: true, // Just modified, so it's active
                                last_message: None, // TODO: Could be extracted for real-time updates
                            };
                            
                            sessions_guard.insert(session_id.clone(), session.clone());
                            
                            // Send appropriate event
                            let cache_event = if is_new {
                                CacheEvent::SessionAdded(session)
                            } else {
                                CacheEvent::SessionModified(session)
                            };
                            
                            let _ = event_tx.send(cache_event);
                        }
                    }
                }
            }
        }
        EventKind::Remove(_) => {
            // Handle file deletion
            for path in event.paths {
                if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                    debug!("JSONL file deleted: {:?}", path);
                    
                    let session_id = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_string();
                    
                    if !session_id.is_empty() {
                        let mut sessions_guard = sessions.write().await;
                        if sessions_guard.remove(&session_id).is_some() {
                            let _ = event_tx.send(CacheEvent::SessionDeleted(session_id));
                        }
                    }
                }
            }
        }
        _ => {}
    }
    
    Ok(())
}