use std::hint::black_box;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct SessionEvent {
    #[serde(rename = "RawPtyOutput")]
    raw_pty_output: Option<RawPtyOutput>,
}

#[derive(Debug, Deserialize)]
struct RawPtyOutput {
    timestamp: u64,
    data: Vec<u8>,
}

fn main() {
    println!("🚀 REAL SESSION TERMINAL PARSING BENCHMARK\n");

    // Load real session data
    let session_data = load_session_data();
    println!("Loaded {} bytes from real session", session_data.len());
    
    bench_vt100_real_session(&session_data);
    println!();
    bench_alacritty_real_session(&session_data);
    println!();
    bench_alacritty_chunked_real_session(&session_data);
    
    println!("\n✅ REAL SESSION BENCHMARK COMPLETE");
}

fn load_session_data() -> Vec<u8> {
    let jsonl_content = std::fs::read_to_string("test_session.jsonl")
        .expect("Failed to read test_session.jsonl");
    
    let mut all_data = Vec::new();
    
    for line in jsonl_content.lines() {
        if line.trim().is_empty() || !line.contains("RawPtyOutput") {
            continue;
        }
        
        // Parse the line as a generic JSON value first
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
            if let Some(raw_output) = value.get("RawPtyOutput") {
                if let Some(data) = raw_output.get("data") {
                    if let Some(array) = data.as_array() {
                        for item in array {
                            if let Some(byte) = item.as_u64() {
                                all_data.push(byte as u8);
                            }
                        }
                    }
                }
            }
        }
    }
    
    all_data
}

fn bench_vt100_real_session(test_data: &[u8]) {
    let iterations = 100_usize;
    
    println!("=== VT100 REAL SESSION BENCHMARK ===");
    println!("Session data size: {} bytes", test_data.len());
    println!("Iterations: {}", iterations);
    
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let mut parser = vt100::Parser::new(30, 80, 1000);
        
        // Process real session data
        parser.process(test_data);
        black_box(&parser);
        
        // Access final state
        let screen = parser.screen();
        black_box(screen.cell(0, 0));
        black_box(screen.cursor_position());
        black_box(screen.scrollback());
    }
    
    let vt100_duration = start.elapsed();
    println!("VT100 total time: {:?}", vt100_duration);
    println!("VT100 per iteration: {:?}", vt100_duration / iterations as u32);
    println!("VT100 bytes/sec: {:.0}", (test_data.len() * iterations) as f64 / vt100_duration.as_secs_f64());
}

fn bench_alacritty_real_session(test_data: &[u8]) {
    let iterations = 100_usize;
    
    println!("=== ALACRITTY REAL SESSION BENCHMARK ===");
    println!("Session data size: {} bytes", test_data.len());
    println!("Iterations: {}", iterations);
    
    use alacritty_terminal::{
        event::VoidListener,
        term::{test::TermSize, Config as TermConfig},
        Term,
    };
    
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let size = TermSize::new(80, 30); // cols, rows
        let mut term = Term::new(TermConfig::default(), &size, VoidListener);
        let mut parser: alacritty_terminal::vte::ansi::Processor = 
            alacritty_terminal::vte::ansi::Processor::new();
        
        // Process real session data byte by byte
        for &byte in test_data {
            parser.advance(&mut term, byte);
            black_box(&parser);
        }
        
        // Access final state
        let grid = term.grid();
        black_box(&grid);
        black_box(grid.cursor.point);
        black_box(grid.display_offset());
    }
    
    let alacritty_duration = start.elapsed();
    println!("Alacritty total time: {:?}", alacritty_duration);
    println!("Alacritty per iteration: {:?}", alacritty_duration / iterations as u32);
    println!("Alacritty bytes/sec: {:.0}", (test_data.len() * iterations) as f64 / alacritty_duration.as_secs_f64());
}

fn bench_alacritty_chunked_real_session(test_data: &[u8]) {
    let iterations = 100_usize;
    
    println!("=== ALACRITTY CHUNKED REAL SESSION BENCHMARK ===");
    println!("Session data size: {} bytes", test_data.len());
    println!("Iterations: {}", iterations);
    
    use alacritty_terminal::{
        event::VoidListener,
        term::{test::TermSize, Config as TermConfig},
        Term,
    };
    
    let start = std::time::Instant::now();
    
    for _ in 0..iterations {
        let size = TermSize::new(80, 30); // cols, rows
        let mut term = Term::new(TermConfig::default(), &size, VoidListener);
        let mut parser: alacritty_terminal::vte::ansi::Processor = 
            alacritty_terminal::vte::ansi::Processor::new();
        
        // Process real session data in chunks
        let chunk_size = 256;
        for chunk in test_data.chunks(chunk_size) {
            for &byte in chunk {
                parser.advance(&mut term, byte);
            }
            black_box(&parser);
        }
        
        // Access final state
        let grid = term.grid();
        black_box(&grid);
        black_box(grid.cursor.point);
        black_box(grid.display_offset());
    }
    
    let alacritty_chunked_duration = start.elapsed();
    println!("Alacritty chunked total time: {:?}", alacritty_chunked_duration);
    println!("Alacritty chunked per iteration: {:?}", alacritty_chunked_duration / iterations as u32);
    println!("Alacritty chunked bytes/sec: {:.0}", (test_data.len() * iterations) as f64 / alacritty_chunked_duration.as_secs_f64());
    
    println!("\n=== REAL SESSION PERFORMANCE COMPARISON ===");
    println!("Data contains real Claude Code session with ANSI escape sequences, Unicode box drawing, and colors");
}