use std::hint::black_box;

fn main() {
    println!("🚀 STARTING TERMINAL PARSING PERFORMANCE BENCHMARKS\n");

    bench_vt100_processing();
    println!();
    bench_alacritty_processing();
    println!();
    bench_alacritty_chunked_processing();
    println!();
    bench_grid_reading_comparison();

    println!("\n✅ BENCHMARK COMPLETE");
}

// Benchmark data - typical terminal output with ANSI sequences
fn generate_test_data() -> Vec<u8> {
    let mut data = Vec::new();

    // Add various ANSI escape sequences and content
    data.extend_from_slice(b"\x1b[2J\x1b[H"); // Clear screen and home cursor
    data.extend_from_slice(b"\x1b[1;31mError: \x1b[0m"); // Bold red "Error: " then reset
    data.extend_from_slice(b"Something went wrong\r\n");
    data.extend_from_slice(b"\x1b[32mSuccess: \x1b[0m"); // Green "Success: " then reset
    data.extend_from_slice(b"Operation completed\r\n");
    data.extend_from_slice(b"\x1b[1;34m"); // Bold blue
    data.extend_from_slice(b"Loading");
    for _ in 0..10 {
        data.extend_from_slice(b".");
        data.extend_from_slice(b"\x1b[K"); // Clear to end of line
    }
    data.extend_from_slice(b"\x1b[0m\r\n"); // Reset and newline

    // Add some scrolling content
    for i in 1..=50 {
        data.extend_from_slice(format!("Line {}: Some content here\r\n", i).as_bytes());
    }

    // Add cursor positioning
    data.extend_from_slice(b"\x1b[10;5H"); // Position cursor
    data.extend_from_slice(b"\x1b[?25l"); // Hide cursor
    data.extend_from_slice(b"Hidden cursor text");
    data.extend_from_slice(b"\x1b[?25h"); // Show cursor

    data
}

fn bench_vt100_processing() {
    let test_data = generate_test_data();
    let iterations = 1000;

    println!("=== VT100 PROCESSING BENCHMARK ===");
    println!("Test data size: {} bytes", test_data.len());
    println!("Iterations: {}", iterations);

    let start = std::time::Instant::now();

    for _ in 0..iterations {
        let mut parser = vt100::Parser::new(30, 80, 1000);

        // Process data in chunks to simulate real usage
        let chunk_size = 256;
        for chunk in test_data.chunks(chunk_size) {
            parser.process(chunk);
            black_box(&parser);
        }

        // Access final state
        let screen = parser.screen();
        black_box(screen.cell(0, 0));
        black_box(screen.cursor_position());
        black_box(screen.scrollback());
    }

    let vt100_duration = start.elapsed();
    println!("VT100 total time: {:?}", vt100_duration);
    println!("VT100 per iteration: {:?}", vt100_duration / iterations);
}

fn bench_alacritty_processing() {
    let test_data = generate_test_data();
    let iterations = 1000;

    println!("=== ALACRITTY TERMINAL PROCESSING BENCHMARK ===");
    println!("Test data size: {} bytes", test_data.len());
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

        // Process data byte by byte (current approach)
        for &byte in &test_data {
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
    println!(
        "Alacritty per iteration: {:?}",
        alacritty_duration / iterations
    );
}

fn bench_alacritty_chunked_processing() {
    let test_data = generate_test_data();
    let iterations = 1000;

    println!("=== ALACRITTY CHUNKED PROCESSING BENCHMARK ===");
    println!("Test data size: {} bytes", test_data.len());
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

        // Process data in chunks (potential optimization)
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
    println!(
        "Alacritty chunked total time: {:?}",
        alacritty_chunked_duration
    );
    println!(
        "Alacritty chunked per iteration: {:?}",
        alacritty_chunked_duration / iterations
    );
}

fn bench_grid_reading_comparison() {
    println!("=== GRID READING COMPARISON ===");

    let test_data = generate_test_data();
    let iterations = 100;

    // Setup VT100
    let mut vt100_parser = vt100::Parser::new(30, 80, 1000);
    vt100_parser.process(&test_data);

    // Setup Alacritty
    use alacritty_terminal::{
        event::VoidListener,
        term::{test::TermSize, Config as TermConfig},
        Term,
    };

    let size = TermSize::new(80, 30);
    let mut alacritty_term = Term::new(TermConfig::default(), &size, VoidListener);
    let mut alacritty_parser: alacritty_terminal::vte::ansi::Processor =
        alacritty_terminal::vte::ansi::Processor::new();

    for &byte in &test_data {
        alacritty_parser.advance(&mut alacritty_term, byte);
    }

    println!("Grid reading iterations: {}", iterations);

    // Benchmark VT100 grid reading
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let screen = vt100_parser.screen();
        for row in 0..30 {
            for col in 0..80 {
                if let Some(cell) = screen.cell(row, col) {
                    black_box(cell.contents());
                    black_box(cell.fgcolor());
                    black_box(cell.bgcolor());
                    black_box(cell.bold());
                }
            }
        }
    }
    let vt100_read_time = start.elapsed();
    println!("VT100 grid reading: {:?}", vt100_read_time);

    // Benchmark Alacritty direct grid reading
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let grid = alacritty_term.grid();
        for row in 0..30 {
            for col in 0..80 {
                let point = alacritty_terminal::index::Point::new(
                    alacritty_terminal::index::Line(row),
                    alacritty_terminal::index::Column(col),
                );
                let cell = &grid[point];
                black_box(cell.c);
                black_box(cell.fg);
                black_box(cell.bg);
                black_box(
                    cell.flags
                        .contains(alacritty_terminal::term::cell::Flags::BOLD),
                );
            }
        }
    }
    let alacritty_direct_time = start.elapsed();
    println!("Alacritty direct grid reading: {:?}", alacritty_direct_time);

    // Benchmark Alacritty display_iter reading
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let grid = alacritty_term.grid();
        for indexed in grid.display_iter() {
            let cell = indexed.cell;
            black_box(cell.c);
            black_box(cell.fg);
            black_box(cell.bg);
            black_box(
                cell.flags
                    .contains(alacritty_terminal::term::cell::Flags::BOLD),
            );
        }
    }
    let alacritty_iter_time = start.elapsed();
    println!("Alacritty display_iter reading: {:?}", alacritty_iter_time);

    println!("\nPerformance ratios (smaller is better):");
    println!(
        "VT100 vs Alacritty direct: {:.2}x",
        alacritty_direct_time.as_nanos() as f64 / vt100_read_time.as_nanos() as f64
    );
    println!(
        "VT100 vs Alacritty display_iter: {:.2}x",
        alacritty_iter_time.as_nanos() as f64 / vt100_read_time.as_nanos() as f64
    );
    println!(
        "Alacritty direct vs display_iter: {:.2}x",
        alacritty_iter_time.as_nanos() as f64 / alacritty_direct_time.as_nanos() as f64
    );
}
