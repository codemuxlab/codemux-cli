use anyhow::Result;

/// Test different VT100 chunking strategies to verify cursor positioning
pub fn test_vt100_chunking_strategies(raw_data_sequence: &[Vec<u8>]) -> Result<()> {
    println!("🧪 Testing AGGRESSIVE VT100 Chunking Strategies");
    println!("Data sequence has {} chunks", raw_data_sequence.len());

    // Add timing analysis
    let _start_time = std::time::Instant::now();

    // Strategy 1: BYTE-BY-BYTE processing (ultra fine-grained)
    let mut byte_parser = vt100::Parser::new(30, 120, 0);
    let mut byte_cursor_history = Vec::new();

    println!("\n📊 Strategy 1: BYTE-BY-BYTE Processing (Ultra fine-grained)");
    let mut all_bytes = Vec::new();
    for data in raw_data_sequence {
        all_bytes.extend_from_slice(data);
    }

    for (i, &byte) in all_bytes.iter().enumerate().take(100) {
        // Limit to first 100 bytes
        byte_parser.process(&[byte]);
        let cursor = byte_parser.screen().cursor_position();
        byte_cursor_history.push((i, cursor));

        let byte_preview = if byte.is_ascii_graphic() || byte == b' ' {
            format!("'{}'", byte as char)
        } else {
            format!("\\x{:02x}", byte)
        };

        if i < 20 || cursor.0 >= 10 {
            // Show first 20 or when cursor gets to input area
            println!(
                "  Byte {}: cursor=({},{}) byte={}",
                i, cursor.0, cursor.1, byte_preview
            );
        }
    }

    // Strategy 2: LINE-BY-LINE processing (split on newlines)
    let mut line_parser = vt100::Parser::new(30, 120, 0);
    let mut line_cursor_history = Vec::new();

    println!("\n📊 Strategy 2: LINE-BY-LINE Processing (Split on newlines)");
    let lines = split_by_lines(raw_data_sequence);
    for (i, data) in lines.iter().enumerate() {
        line_parser.process(data);
        let cursor = line_parser.screen().cursor_position();
        line_cursor_history.push((i, cursor));

        let data_preview: String = String::from_utf8_lossy(data)
            .chars()
            .map(|c| {
                if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                    format!("\\x{:02x}", c as u8)
                } else {
                    c.to_string()
                }
            })
            .take(50)
            .collect();

        println!(
            "  Line {}: cursor=({},{}) data={}",
            i, cursor.0, cursor.1, data_preview
        );
    }

    // Strategy 3: ESCAPE-SEQUENCE-AWARE processing (split at escape sequences)
    let mut escape_parser = vt100::Parser::new(30, 120, 0);
    let mut escape_cursor_history = Vec::new();

    println!("\n📊 Strategy 3: ESCAPE-SEQUENCE-AWARE Processing");
    let escape_chunks = split_by_escape_sequences(raw_data_sequence);
    for (i, data) in escape_chunks.iter().enumerate() {
        escape_parser.process(data);
        let cursor = escape_parser.screen().cursor_position();
        escape_cursor_history.push((i, cursor));

        let data_preview: String = String::from_utf8_lossy(data)
            .chars()
            .map(|c| {
                if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                    format!("\\x{:02x}", c as u8)
                } else {
                    c.to_string()
                }
            })
            .take(50)
            .collect();

        println!(
            "  Escape {}: cursor=({},{}) data={}",
            i, cursor.0, cursor.1, data_preview
        );
    }

    // Strategy 4: REVERSE ORDER processing (process from end to start)
    let mut reverse_parser = vt100::Parser::new(30, 120, 0);
    let mut reverse_cursor_history = Vec::new();

    println!("\n📊 Strategy 4: REVERSE ORDER Processing");
    for (i, data) in raw_data_sequence.iter().rev().enumerate() {
        reverse_parser.process(data);
        let cursor = reverse_parser.screen().cursor_position();
        reverse_cursor_history.push((i, cursor));

        let data_preview: String = String::from_utf8_lossy(data)
            .chars()
            .map(|c| {
                if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                    format!("\\x{:02x}", c as u8)
                } else {
                    c.to_string()
                }
            })
            .take(50)
            .collect();

        println!(
            "  Reverse {}: cursor=({},{}) data={}",
            i, cursor.0, cursor.1, data_preview
        );
    }

    // Strategy 5: RANDOM ORDER processing (shuffle chunks)
    let mut random_parser = vt100::Parser::new(30, 120, 0);
    let mut random_cursor_history = Vec::new();

    println!("\n📊 Strategy 5: RANDOM ORDER Processing");
    let mut shuffled_indices: Vec<usize> = (0..raw_data_sequence.len()).collect();
    // Simple shuffle using XOR-shift
    let mut seed = 123456789u32;
    for i in (1..shuffled_indices.len()).rev() {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        let j = (seed as usize) % (i + 1);
        shuffled_indices.swap(i, j);
    }

    for (order, &original_index) in shuffled_indices.iter().enumerate().take(10) {
        // Limit to first 10
        let data = &raw_data_sequence[original_index];
        random_parser.process(data);
        let cursor = random_parser.screen().cursor_position();
        random_cursor_history.push((order, cursor));

        let data_preview: String = String::from_utf8_lossy(data)
            .chars()
            .map(|c| {
                if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                    format!("\\x{:02x}", c as u8)
                } else {
                    c.to_string()
                }
            })
            .take(30)
            .collect();

        println!(
            "  Random {}: cursor=({},{}) orig_idx={} data={}",
            order, cursor.0, cursor.1, original_index, data_preview
        );
    }

    // Strategy 6: SKIP STATUS UPDATES processing (filter out status-related chunks)
    let mut filtered_parser = vt100::Parser::new(30, 120, 0);
    let mut filtered_cursor_history = Vec::new();

    println!("\n📊 Strategy 6: SKIP STATUS UPDATES Processing");
    for (i, data) in raw_data_sequence.iter().enumerate() {
        let data_str = String::from_utf8_lossy(data);

        // Skip chunks that contain status update patterns
        if data_str.contains("Claude") && data_str.contains("limit") ||
           data_str.contains("\x1b[12;1H") || // Move to status area
           data_str.contains("\x1b[2K\x1b[1A")
        // Clear line + cursor up
        {
            println!(
                "  Skipping status chunk {}: {}",
                i,
                data_str
                    .chars()
                    .take(30)
                    .collect::<String>()
                    .replace('\x1b', "\\x1b")
            );
            continue;
        }

        filtered_parser.process(data);
        let cursor = filtered_parser.screen().cursor_position();
        filtered_cursor_history.push((i, cursor));

        let data_preview: String = data_str
            .chars()
            .map(|c| {
                if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                    format!("\\x{:02x}", c as u8)
                } else {
                    c.to_string()
                }
            })
            .take(50)
            .collect();

        println!(
            "  Filtered {}: cursor=({},{}) data={}",
            i, cursor.0, cursor.1, data_preview
        );
    }

    // Strategy 7: IMMEDIATE processing (like capture system) - for comparison
    let mut immediate_parser = vt100::Parser::new(30, 120, 0);
    let mut immediate_cursor_history = Vec::new();

    println!("\n📊 Strategy 7: IMMEDIATE Processing (Original Capture-style)");
    for (i, data) in raw_data_sequence.iter().enumerate() {
        immediate_parser.process(data);
        let cursor = immediate_parser.screen().cursor_position();
        immediate_cursor_history.push((i, cursor));

        let data_preview: String = String::from_utf8_lossy(data)
            .chars()
            .map(|c| {
                if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                    format!("\\x{:02x}", c as u8)
                } else {
                    c.to_string()
                }
            })
            .take(50)
            .collect();

        if i < 5 || cursor.0 >= 10 {
            // Show first few or when cursor gets to input area
            println!(
                "  Immediate {}: cursor=({},{}) data={}",
                i, cursor.0, cursor.1, data_preview
            );
        }
    }

    // Compare final cursor positions
    println!("\n🎯 Final Cursor Position Comparison:");
    let byte_final = byte_cursor_history
        .last()
        .map(|(_, c)| c)
        .unwrap_or(&(0, 0));
    let line_final = line_cursor_history
        .last()
        .map(|(_, c)| c)
        .unwrap_or(&(0, 0));
    let escape_final = escape_cursor_history
        .last()
        .map(|(_, c)| c)
        .unwrap_or(&(0, 0));
    let reverse_final = reverse_cursor_history
        .last()
        .map(|(_, c)| c)
        .unwrap_or(&(0, 0));
    let random_final = random_cursor_history
        .last()
        .map(|(_, c)| c)
        .unwrap_or(&(0, 0));
    let filtered_final = filtered_cursor_history
        .last()
        .map(|(_, c)| c)
        .unwrap_or(&(0, 0));
    let immediate_final = immediate_cursor_history
        .last()
        .map(|(_, c)| c)
        .unwrap_or(&(0, 0));

    println!("  Byte-by-byte:    ({}, {})", byte_final.0, byte_final.1);
    println!("  Line-by-line:    ({}, {})", line_final.0, line_final.1);
    println!(
        "  Escape-aware:    ({}, {})",
        escape_final.0, escape_final.1
    );
    println!(
        "  Reverse order:   ({}, {})",
        reverse_final.0, reverse_final.1
    );
    println!(
        "  Random order:    ({}, {})",
        random_final.0, random_final.1
    );
    println!(
        "  Skip status:     ({}, {})",
        filtered_final.0, filtered_final.1
    );
    println!(
        "  Immediate:       ({}, {})",
        immediate_final.0, immediate_final.1
    );

    // Find strategies that get cursor to ACTUAL input position (row 8-12, col 4+)
    println!("\n✨ Strategies with cursor in ACTUAL INPUT POSITION (row 8-12, col 4+):");
    let strategies = [
        ("Byte-by-byte", byte_final),
        ("Line-by-line", line_final),
        ("Escape-aware", escape_final),
        ("Reverse order", reverse_final),
        ("Random order", random_final),
        ("Skip status", filtered_final),
        ("Immediate", immediate_final),
    ];

    let mut found_good_strategy = false;
    for (name, cursor) in strategies {
        if cursor.0 >= 8 && cursor.0 <= 12 && cursor.1 >= 4 {
            println!(
                "  🎯 {}: cursor=({},{}) ← ACTUAL INPUT POSITION!",
                name, cursor.0, cursor.1
            );
            found_good_strategy = true;
        } else if cursor.0 >= 8 && cursor.0 <= 12 && cursor.1 == 0 {
            println!(
                "  ⚠️  {}: cursor=({},{}) ← WRONG COLUMN (should be 4+)",
                name, cursor.0, cursor.1
            );
        } else {
            println!(
                "  ❌ {}: cursor=({},{}) ← WRONG AREA",
                name, cursor.0, cursor.1
            );
        }
    }

    if !found_good_strategy {
        println!("  ❌ No strategy got cursor to actual input position (row 8-12, col 4+)");
        println!("  💡 All cursors at column 0 - this is the problem! Should be column 4+ for text input.");
    }

    // Analyze most promising strategy
    if filtered_final.0 >= 10 {
        println!("\n🔍 SKIP STATUS strategy shows promise - analyzing cursor movement:");
        for (i, cursor) in filtered_cursor_history.iter().take(10) {
            println!("    Step {}: ({}, {})", i, cursor.0, cursor.1);
        }
    }

    Ok(())
}

/// Split data by line boundaries (newlines)
fn split_by_lines(raw_data_sequence: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut result = Vec::new();
    let mut current_line = Vec::new();

    for data in raw_data_sequence {
        for &byte in data {
            current_line.push(byte);
            if (byte == b'\n' || byte == b'\r') && !current_line.is_empty() {
                result.push(current_line.clone());
                current_line.clear();
            }
        }
    }

    // Add any remaining data
    if !current_line.is_empty() {
        result.push(current_line);
    }

    result
}

/// Split data by escape sequence boundaries
fn split_by_escape_sequences(raw_data_sequence: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut result = Vec::new();
    let mut current_chunk = Vec::new();
    let mut in_escape_sequence = false;

    for data in raw_data_sequence {
        for &byte in data {
            current_chunk.push(byte);

            if byte == 0x1b {
                // ESC character
                if !current_chunk.is_empty() && current_chunk.len() > 1 {
                    // End previous chunk before escape
                    let mut prev_chunk = current_chunk.clone();
                    prev_chunk.pop(); // Remove the ESC we just added
                    if !prev_chunk.is_empty() {
                        result.push(prev_chunk);
                    }
                    current_chunk = vec![byte]; // Start new chunk with ESC
                }
                in_escape_sequence = true;
            } else if in_escape_sequence && (byte.is_ascii_alphabetic() || byte == b'~') {
                // End of escape sequence
                in_escape_sequence = false;
                if !current_chunk.is_empty() {
                    result.push(current_chunk.clone());
                    current_chunk.clear();
                }
            }
        }
    }

    // Add any remaining data
    if !current_chunk.is_empty() {
        result.push(current_chunk);
    }

    result
}



/// Load raw data sequence from a JSONL file for testing
pub fn load_test_data_from_jsonl(jsonl_path: &str) -> Result<Vec<Vec<u8>>> {
    use crate::capture::session_data::SessionEvent;
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let file = File::open(jsonl_path)?;
    let reader = BufReader::new(file);
    let mut raw_data_sequence = Vec::new();

    for line in reader.lines().skip(1) {
        // Skip metadata line
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Try to parse with new format first, fall back to old format
        if let Ok(event) = serde_json::from_str::<SessionEvent>(&line) {
            if let SessionEvent::RawPtyOutput { data, .. } = event {
                raw_data_sequence.push(data);
            }
        } else {
            // Handle old format by manually parsing JSON
            if line.contains("\"RawPtyOutput\"") {
                if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&line) {
                    if let Some(raw_pty) = json_value.get("RawPtyOutput") {
                        if let Some(data_array) = raw_pty.get("data") {
                            if let Some(data_vec) = data_array.as_array() {
                                let data: Vec<u8> = data_vec
                                    .iter()
                                    .filter_map(|v| v.as_u64().map(|n| n as u8))
                                    .collect();
                                raw_data_sequence.push(data);
                            }
                        }
                    }
                }
            }
        }
    }

    println!(
        "Loaded {} raw data chunks from {}",
        raw_data_sequence.len(),
        jsonl_path
    );
    Ok(raw_data_sequence)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_chunking_difference() {
        // Create a simple test case that should show cursor differences
        let raw_data = vec![
            // Step 1: Move to status area
            b"\x1b[12;1H".to_vec(), // Move cursor to row 12, col 1
            b"Status message".to_vec(),
            // Step 2: Clear and move up
            b"\x1b[2K\x1b[1A\x1b[2K\x1b[1A".to_vec(),
            b"\x1b[G".to_vec(), // Move to column 0
            // Step 3: Redraw input
            b"> \x1b[7mT\x1b[27m".to_vec(), // "> " + highlighted T
        ];

        test_vt100_chunking_strategies(&raw_data).unwrap();
    }
}
