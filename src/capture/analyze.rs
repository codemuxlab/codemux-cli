use anyhow::Result;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::capture::session_data::SessionEvent;

pub async fn analyze_jsonl_data(input_path: &Path, verbose: bool) -> Result<()> {
    println!("📊 Loading JSONL data from: {}", input_path.display());

    let file = File::open(input_path)?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();

    // Skip metadata line
    if let Some(metadata_line) = lines.next() {
        println!("📋 Metadata: {}", metadata_line?);
    }

    // Compare two VT100 processing approaches
    let mut incremental_parser = vt100::Parser::new(30, 120, 0);
    let mut batched_parser = vt100::Parser::new(30, 120, 0);
    let mut batched_data = Vec::new();

    let mut event_count = 0;
    let mut cursor_differences = Vec::new();
    let mut all_events = Vec::new(); // Track all events for sequence analysis

    println!("🔄 Processing events...");
    println!("Method comparison: INCREMENTAL vs BATCHED");
    println!(
        "{:<6} {:<8} {:<12} {:<20} {:<20} {:<10}",
        "Event", "Time(ms)", "Type", "Incremental Cursor", "Batched Cursor", "Diff?"
    );
    println!("{}", "-".repeat(90));

    for line in lines {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let event: SessionEvent = serde_json::from_str(&line)?;
        event_count += 1;

        match event {
            SessionEvent::RawPtyOutput {
                timestamp_begin,
                data,
                ..
            } => {
                all_events.push((event_count, timestamp_begin, data.clone()));
                // INCREMENTAL: Process immediately (like capture system)
                incremental_parser.process(&data);
                let incremental_cursor = incremental_parser.screen().cursor_position();

                // BATCHED: Accumulate data (like main TUI system)
                batched_data.extend_from_slice(&data);

                // Process batched data every few events to simulate debouncing
                let batched_cursor = if event_count % 3 == 0 || data.len() > 100 {
                    batched_parser.process(&batched_data);
                    let cursor = batched_parser.screen().cursor_position();
                    batched_data.clear();
                    cursor
                } else {
                    batched_parser.screen().cursor_position()
                };

                let cursor_diff = incremental_cursor != batched_cursor;
                if cursor_diff || verbose {
                    println!(
                        "{:<6} {:<8} {:<12} ({:>2},{:>3})          ({:>2},{:>3})          {:<10}",
                        event_count,
                        timestamp_begin,
                        "RawOutput",
                        incremental_cursor.0,
                        incremental_cursor.1,
                        batched_cursor.0,
                        batched_cursor.1,
                        if cursor_diff { "❌ DIFF" } else { "✅ Same" }
                    );

                    // Analyze the input stream in detail
                    analyze_input_stream(&data, incremental_cursor, batched_cursor);
                }

                if cursor_diff {
                    cursor_differences.push((
                        timestamp_begin,
                        incremental_cursor,
                        batched_cursor,
                        data.clone(),
                    ));
                }

                if verbose && !data.is_empty() {
                    let printable: String = String::from_utf8_lossy(&data)
                        .chars()
                        .map(|c| {
                            if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                                format!("\\x{:02x}", c as u8)
                            } else {
                                c.to_string()
                            }
                        })
                        .collect();
                    println!("      Data: {}", printable);
                }
            }
            SessionEvent::GridUpdate {
                timestamp_begin,
                cursor,
                ..
            } => {
                if verbose {
                    println!(
                        "{:<6} {:<8} {:<12} Grid cursor: ({:>2},{:>3})",
                        event_count, timestamp_begin, "GridUpdate", cursor.0, cursor.1
                    );
                }
            }
            _ => {
                if verbose {
                    println!("{:<6} {:<8} {:<12} (skipped)", event_count, "N/A", "Other");
                }
            }
        }
    }

    // Process any remaining batched data
    if !batched_data.is_empty() {
        batched_parser.process(&batched_data);
    }

    println!("\n📈 Analysis Results:");
    println!("Total events processed: {}", event_count);
    println!("Cursor differences found: {}", cursor_differences.len());

    if !cursor_differences.is_empty() {
        println!("\n🔍 Cursor Difference Details:");
        for (i, (timestamp, inc_cursor, batch_cursor, data)) in
            cursor_differences.iter().enumerate().take(10)
        {
            println!(
                "#{} at {}ms: Incremental({},{}) vs Batched({},{}):",
                i + 1,
                timestamp,
                inc_cursor.0,
                inc_cursor.1,
                batch_cursor.0,
                batch_cursor.1
            );

            let printable: String = String::from_utf8_lossy(data)
                .chars()
                .take(100)
                .map(|c| {
                    if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                        format!("\\x{:02x}", c as u8)
                    } else {
                        c.to_string()
                    }
                })
                .collect();
            println!("   Data: {}", printable);
        }

        if cursor_differences.len() > 10 {
            println!(
                "   ... and {} more differences",
                cursor_differences.len() - 10
            );
        }
    }

    // Final cursor positions
    let final_inc = incremental_parser.screen().cursor_position();
    let final_batch = batched_parser.screen().cursor_position();

    println!("\n🎯 Final Cursor Positions:");
    println!("Incremental processing: ({}, {})", final_inc.0, final_inc.1);
    println!(
        "Batched processing:     ({}, {})",
        final_batch.0, final_batch.1
    );

    if final_inc != final_batch {
        println!("❌ FINAL CURSORS DIFFER - This explains the main TUI cursor issue!");
        println!("💡 The batching/debouncing in main TUI causes different VT100 state than capture system");
    } else {
        println!("✅ Final cursors match - issue might be elsewhere");
    }

    // Analyze timing patterns and suggest better chunking strategies
    analyze_timing_patterns(&cursor_differences).await?;

    // Analyze cursor movement sequences
    analyze_cursor_return_sequences(&all_events, &cursor_differences).await?;

    Ok(())
}

type CursorDifference = (u32, (u16, u16), (u16, u16), Vec<u8>);

async fn analyze_timing_patterns(
    cursor_differences: &[CursorDifference],
) -> Result<()> {
    println!("\n🕒 Timing Analysis & Smart Chunking Suggestions:");

    if cursor_differences.is_empty() {
        println!("No cursor differences to analyze.");
        return Ok(());
    }

    // Analyze timing gaps between problematic events
    let mut timing_gaps = Vec::new();
    for i in 1..cursor_differences.len() {
        let gap = cursor_differences[i].0 - cursor_differences[i - 1].0;
        timing_gaps.push(gap);
    }

    if !timing_gaps.is_empty() {
        let avg_gap = timing_gaps.iter().sum::<u32>() / timing_gaps.len() as u32;
        let min_gap = timing_gaps.iter().min().unwrap();
        let max_gap = timing_gaps.iter().max().unwrap();

        println!("Timing gaps between cursor differences:");
        println!(
            "  Average: {}ms, Min: {}ms, Max: {}ms",
            avg_gap, min_gap, max_gap
        );
    }

    // Analyze VT100 sequence patterns
    println!("\n📋 VT100 Sequence Analysis:");
    let mut sequence_patterns = std::collections::HashMap::new();

    for (_timestamp, _inc_cursor, _batch_cursor, data) in cursor_differences {
        let data_str = String::from_utf8_lossy(data);

        // Look for common VT100 patterns that cause cursor issues
        if data_str.contains("\x1b[2K") && data_str.contains("\x1b[1A") {
            *sequence_patterns
                .entry("screen_clear_sequence")
                .or_insert(0) += 1;
        }
        if data_str.contains("Claude Opus limit reached") {
            *sequence_patterns.entry("status_message").or_insert(0) += 1;
        }
        if data_str.contains("\x1b[G") {
            *sequence_patterns.entry("cursor_column_reset").or_insert(0) += 1;
        }
        if data_str.contains("\x1b[39m\x1b[22m") {
            *sequence_patterns.entry("style_reset").or_insert(0) += 1;
        }
    }

    for (pattern, count) in sequence_patterns.iter() {
        println!("  {}: {} occurrences", pattern, count);
    }

    println!("\n💡 Smart Chunking Recommendations:");

    // Recommendation 1: VT100 sequence boundaries
    println!("1. **VT100 Sequence Boundary Chunking:**");
    println!("   - Don't batch across complete VT100 escape sequences");
    println!("   - Process screen clearing sequences (\\x1b[2K\\x1b[1A...) as atomic units");
    println!("   - Separate cursor positioning from content updates");

    // Recommendation 2: Timing-based chunking
    if !timing_gaps.is_empty() {
        let suggested_timeout = timing_gaps.iter().min().unwrap() / 2;
        println!("2. **Adaptive Timing Chunking:**");
        println!("   - Current debounce timeout might be too long");
        println!(
            "   - Suggested max timeout: {}ms (half of smallest gap)",
            suggested_timeout
        );
        println!("   - Use shorter timeouts during active typing vs idle periods");
    }

    // Recommendation 3: Content-aware chunking
    println!("3. **Content-Aware Chunking:**");
    println!("   - Detect status line updates vs input area updates");
    println!("   - Process status messages immediately (don't batch with user input)");
    println!("   - Separate text input from cursor movement commands");

    // Recommendation 4: Hybrid approach
    println!("4. **Hybrid Processing Approach:**");
    println!("   - Use incremental processing for cursor positioning commands");
    println!("   - Use batching only for pure content updates (text characters)");
    println!("   - Process \\x1b[G (cursor to column 0) immediately, not in batch");

    println!("\n🔧 Recommended Implementation:");
    println!("```rust");
    println!("enum VT100ChunkType {{");
    println!("    CursorMovement,  // Process immediately");
    println!("    StatusUpdate,    // Process immediately  ");
    println!("    TextInput,       // Can be batched with short timeout");
    println!("    StyleChange,     // Can be batched");
    println!("}}");
    println!();
    println!("fn classify_vt100_data(data: &[u8]) -> VT100ChunkType {{");
    println!("    let s = String::from_utf8_lossy(data);");
    println!("    if s.contains(\"\\x1b[G\") || s.contains(\"\\x1b[2K\\x1b[1A\") {{");
    println!("        VT100ChunkType::CursorMovement");
    println!("    }} else if s.contains(\"Claude\") && s.contains(\"limit\") {{");
    println!("        VT100ChunkType::StatusUpdate");
    println!("    }} else {{");
    println!("        VT100ChunkType::TextInput");
    println!("    }}");
    println!("}}");
    println!("```");

    Ok(())
}

async fn analyze_cursor_return_sequences(
    all_events: &[(usize, u32, Vec<u8>)],
    _cursor_differences: &[CursorDifference],
) -> Result<()> {
    println!("\n🔍 Cursor Return Sequence Analysis:");
    println!("Analyzing what happens AFTER status messages to move cursor back to input area...");

    // Find events that contain status messages
    let mut status_message_events = Vec::new();
    for (event_num, _timestamp, data) in all_events {
        let data_str = String::from_utf8_lossy(data);
        if data_str.contains("Claude") && data_str.contains("limit") {
            status_message_events.push(*event_num);
        }
    }

    println!(
        "Found {} status message events at positions: {:?}",
        status_message_events.len(),
        status_message_events
    );

    // Analyze the 3-5 events AFTER each status message to see cursor movement patterns
    for status_event_num in status_message_events.iter().take(3) {
        // Analyze first 3 for brevity
        println!(
            "\n📋 Events after status message at event #{}:",
            status_event_num
        );

        // Look at next 5 events
        for i in 1..=5 {
            let next_event_num = status_event_num + i;
            if let Some((_, timestamp, data)) =
                all_events.iter().find(|(num, _, _)| *num == next_event_num)
            {
                let data_str = String::from_utf8_lossy(data);

                println!("  Event #{}: {}ms", next_event_num, timestamp);

                // Show key VT100 sequences that move cursor
                if data_str.contains("\x1b[2K\x1b[1A") {
                    println!("    🧹⬆️ Clear line & cursor up sequence");
                }
                if data_str.contains("\x1b[G") {
                    println!("    🔄 Move cursor to column 0");
                }
                if data_str.contains("\x1b[7m") {
                    println!("    🔦 Highlight cursor (reverse video)");
                }

                // Look for text input area recreation
                if data_str.contains("> ") && data_str.contains("\x1b[7m \x1b[27m") {
                    println!("    ✨ INPUT AREA RECREATION: Creates '> [cursor] ' prompt");
                }

                // Show a snippet of the data
                let preview: String = data_str
                    .chars()
                    .map(|c| {
                        if c.is_control() && c != '\n' && c != '\r' && c != '\t' {
                            format!("\\x{:02x}", c as u8)
                        } else {
                            c.to_string()
                        }
                    })
                    .take(80)
                    .collect();
                println!("    Data: {}", preview);
            }
        }
    }

    println!("\n💡 Cursor Return Mechanism Analysis:");
    println!("After displaying status message at (12,0), Claude likely:");
    println!("1. Clears multiple lines with \\x1b[2K\\x1b[1A sequences (move up & clear)");
    println!("2. Moves cursor to column 0 with \\x1b[G");
    println!("3. Redraws the input prompt with highlighted cursor: '> \\x1b[7m \\x1b[27m'");
    println!("4. This places cursor back in input area around (10, 115-118)");
    println!("\nThe batched system misses these intermediate cursor movements!");

    Ok(())
}

fn analyze_input_stream(data: &[u8], incremental_cursor: (u16, u16), batched_cursor: (u16, u16)) {
    let data_str = String::from_utf8_lossy(data);

    // Look for input patterns that contain user typing
    if data_str.contains("> ")
        && (data_str.contains("t") || data_str.contains("e") || data_str.contains("s"))
    {
        println!("      📝 INPUT ANALYSIS:");

        // Find the input line (look for "> " pattern)
        if let Some(input_start) = data_str.find("> ") {
            let input_line = &data_str[input_start..];
            if let Some(input_end) = input_line.find('\n').or(input_line.find("│")) {
                let input_content = &input_line[..input_end];

                // Parse the input content to find actual typed text vs suggestions
                println!(
                    "      Input line: {}",
                    input_content.chars().take(80).collect::<String>()
                );

                // Count actual characters (excluding ANSI sequences)
                let mut actual_chars = 0;
                let mut in_escape = false;
                let mut typed_text = String::new();

                for c in input_content.chars().skip(2) {
                    // Skip "> "
                    if c == '\x1b' {
                        in_escape = true;
                        continue;
                    }
                    if in_escape {
                        if c.is_alphabetic() || c == 'm' {
                            in_escape = false;
                        }
                        continue;
                    }

                    // Look for the cursor highlight pattern
                    if !c.is_control() {
                        typed_text.push(c);
                        actual_chars += 1;

                        // Stop at grayed text or after highlighted cursor
                        if typed_text.len() > 10 {
                            // Reasonable limit for typed text
                            break;
                        }
                    }
                }

                let expected_cursor_col = 2 + actual_chars; // "> " + typed chars
                println!(
                    "      Typed text: '{}' ({} chars)",
                    typed_text, actual_chars
                );
                println!(
                    "      Expected cursor column: {} (after '> {}')",
                    expected_cursor_col, typed_text
                );
                println!(
                    "      Incremental cursor: ({}, {}) - {} columns from expected",
                    incremental_cursor.0,
                    incremental_cursor.1,
                    (incremental_cursor.1 as i32 - expected_cursor_col).abs()
                );
                println!(
                    "      Batched cursor:     ({}, {}) - {} columns from expected",
                    batched_cursor.0,
                    batched_cursor.1,
                    (batched_cursor.1 as i32 - expected_cursor_col).abs()
                );
            }
        }
    }

    // Show VT100 sequences
    if data_str.contains("\x1b[G") {
        println!("      🔄 Contains \\x1b[G (move cursor to column 0)");
    }
    if data_str.contains("\x1b[2K") {
        println!("      🧹 Contains \\x1b[2K (clear line)");
    }
    if data_str.contains("\x1b[1A") {
        println!("      ⬆️ Contains \\x1b[1A (cursor up)");
    }
    if data_str.contains("\x1b[7m") {
        println!("      🔦 Contains \\x1b[7m (reverse video - cursor highlight)");
    }
    if data_str.contains("\x1b[27m") {
        println!("      💡 Contains \\x1b[27m (normal video - cursor unhighlight)");
    }
}
