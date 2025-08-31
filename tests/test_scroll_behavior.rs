#[cfg(test)]
mod tests {
    use alacritty_terminal::{
        event::VoidListener,
        grid::Dimensions,
        index::{Column, Line, Point},
        term::{test::TermSize, Config as TermConfig},
        Term,
    };

    #[test]
    fn test_scroll_behavior_with_small_terminal() {
        println!("=== TESTING SCROLL BEHAVIOR WITH SMALL TERMINAL ===\n");

        // Create a very small terminal to force scrolling
        let size = TermSize::new(40, 5); // 40 columns, 5 rows
        let mut term = Term::new(TermConfig::default(), &size, VoidListener);
        
        // Initialize VTE parser
        use alacritty_terminal::vte::ansi;
        let mut parser: ansi::Processor = ansi::Processor::new();

        println!("🏗️ Created small terminal: {}x{}", size.columns(), size.screen_lines());
        
        // Fill terminal with content to force scrolling
        let test_lines = vec![
            "Line 1: This is the first line of text",
            "Line 2: This is the second line of text", 
            "Line 3: This is the third line of text",
            "Line 4: This is the fourth line of text",
            "Line 5: This is the fifth line of text",
            "Line 6: This is the sixth line (should scroll)",
            "Line 7: This is the seventh line (should scroll)",
            "Line 8: This is the eighth line (should scroll)",
        ];

        // Feed content through VTE parser
        println!("\n📝 Adding content to force scrolling:");
        for (i, line) in test_lines.iter().enumerate() {
            println!("   Adding line {}: {}", i + 1, line);
            
            // Add the line content
            for &byte in line.as_bytes() {
                parser.advance(&mut term, byte);
            }
            // Add newline
            parser.advance(&mut term, b'\r');
            parser.advance(&mut term, b'\n');
            
            // Check current state after each line
            let grid = term.grid();
            let cursor = grid.cursor.point;
            let display_offset = grid.display_offset();
            let total_lines = term.total_lines();
            
            println!("     Cursor: ({}, {}), Display offset: {}, Total lines: {}", 
                cursor.line.0, cursor.column.0, display_offset, total_lines);
        }

        // Check final state
        let grid = term.grid();
        let display_offset = grid.display_offset();
        let total_lines = term.total_lines();
        let screen_lines = term.screen_lines();
        
        println!("\n📊 Final terminal state:");
        println!("   Screen lines: {}", screen_lines);
        println!("   Total lines: {}", total_lines);
        println!("   Display offset: {}", display_offset);
        println!("   Cursor: ({}, {})", grid.cursor.point.line.0, grid.cursor.point.column.0);
        
        // Print current visible content
        println!("\n📋 Current visible content:");
        for row in 0..screen_lines.min(5) {
            let mut line_content = String::new();
            for col in 0..40.min(grid.columns()) {
                let point = Point::new(Line(row as i32), Column(col));
                let cell = &grid[point];
                if cell.c != ' ' {
                    line_content.push(cell.c);
                } else if !line_content.is_empty() {
                    line_content.push(' ');
                }
            }
            println!("   Row {}: '{}'", row, line_content.trim_end());
        }

        // Test scrolling up
        println!("\n⬆️ Testing scroll up:");
        use alacritty_terminal::grid::Scroll;
        term.scroll_display(Scroll::Delta(2)); // Scroll up 2 lines
        
        let new_display_offset = term.grid().display_offset();
        println!("   New display offset after scroll up: {}", new_display_offset);
        
        // Print content after scroll up
        println!("   Content after scroll up:");
        let grid = term.grid();
        for row in 0..screen_lines.min(5) {
            let mut line_content = String::new();
            for col in 0..40.min(grid.columns()) {
                let point = Point::new(Line(row as i32), Column(col));
                let cell = &grid[point];
                if cell.c != ' ' {
                    line_content.push(cell.c);
                } else if !line_content.is_empty() {
                    line_content.push(' ');
                }
            }
            println!("     Row {}: '{}'", row, line_content.trim_end());
        }

        // Test scrolling down
        println!("\n⬇️ Testing scroll down:");
        term.scroll_display(Scroll::Delta(-1)); // Scroll down 1 line
        
        let final_display_offset = term.grid().display_offset();
        println!("   Final display offset after scroll down: {}", final_display_offset);
        
        // Print content after scroll down
        println!("   Content after scroll down:");
        let grid = term.grid();
        for row in 0..screen_lines.min(5) {
            let mut line_content = String::new();
            for col in 0..40.min(grid.columns()) {
                let point = Point::new(Line(row as i32), Column(col));
                let cell = &grid[point];
                if cell.c != ' ' {
                    line_content.push(cell.c);
                } else if !line_content.is_empty() {
                    line_content.push(' ');
                }
            }
            println!("     Row {}: '{}'", row, line_content.trim_end());
        }

        // Test reset to bottom
        println!("\n⬇️ Testing reset to bottom:");
        term.scroll_display(Scroll::Bottom);
        
        let bottom_display_offset = term.grid().display_offset();
        println!("   Display offset after reset to bottom: {}", bottom_display_offset);
        
        println!("\n✅ Scroll test completed!");
    }
}