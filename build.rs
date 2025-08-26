use std::env;
use std::path::Path;
use std::process::Command;

fn find_npm_command() -> &'static str {
    if cfg!(target_os = "windows") {
        // On Windows, try npm.cmd first (standard location), then npm
        if Command::new("npm.cmd").arg("--version").output().is_ok() {
            "npm.cmd"
        } else if Command::new("npm").arg("--version").output().is_ok() {
            "npm"
        } else {
            // Try common Windows npm locations
            let potential_paths = [
                "C:\\Program Files\\nodejs\\npm.cmd",
                "C:\\Program Files (x86)\\nodejs\\npm.cmd",
                "C:\\ProgramData\\chocolatey\\lib\\nodejs\\tools\\npm.cmd",
            ];
            
            for path in &potential_paths {
                if Path::new(path).exists() {
                    return path;
                }
            }
            "npm" // fallback
        }
    } else {
        "npm"
    }
}

fn main() {
    // Skip React Native build if SKIP_WEB_BUILD is set
    if env::var("SKIP_WEB_BUILD").is_ok() {
        println!("cargo:warning=Skipping React Native Web build (SKIP_WEB_BUILD set)");
        return;
    }

    // Watch for changes in the React app
    println!("cargo:rerun-if-changed=app/src");
    println!("cargo:rerun-if-changed=app/package.json");
    println!("cargo:rerun-if-changed=app/app.json");
    println!("cargo:rerun-if-changed=app/tsconfig.json");
    println!("cargo:rerun-if-changed=app/tailwind.config.js");

    // Only build the React app in release mode or when explicitly requested
    let should_build_app = env::var("CARGO_FEATURE_BUILD_APP").is_ok()
        || env::var("PROFILE").unwrap_or_default() == "release"
        || env::var("CODEMUX_BUILD_APP").is_ok();

    if !should_build_app {
        println!("cargo:warning=Skipping React Native Web build (use CODEMUX_BUILD_APP=1 or cargo build --release)");
        return;
    }

    // Check if we're in the right directory and have required files
    if !Path::new("app/package.json").exists() {
        println!("cargo:warning=No app/package.json found, skipping React Native Web build");
        return;
    }

    // Check if node_modules exists, if not install dependencies
    if !Path::new("app/node_modules").exists() {
        println!("cargo:warning=Installing React Native Web dependencies...");

        let npm_cmd = find_npm_command();
        println!("cargo:warning=Using npm command: {}", npm_cmd);

        let npm_install = Command::new(npm_cmd)
            .args(["install"])
            .current_dir("app")
            .output()
            .expect("Failed to execute npm install");

        if !npm_install.status.success() {
            panic!(
                "Failed to install React app dependencies:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&npm_install.stdout),
                String::from_utf8_lossy(&npm_install.stderr)
            );
        }
    }

    println!("cargo:warning=Building React Native Web app (this may take a while)...");

    // Build the React Native Web app
    let npm_cmd = find_npm_command();
    let output = Command::new(npm_cmd)
        .args(["run", "build"])
        .current_dir("app")
        .output()
        .expect("Failed to execute npm run build");

    if !output.status.success() {
        panic!(
            "Failed to build React Native Web app:\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
    }

    println!("cargo:warning=React Native Web app built successfully");

    // Check that the dist directory was created
    if !Path::new("app/dist").exists() {
        panic!("React Native Web build completed but no dist directory found");
    }

    // Tell cargo where to find the built assets
    println!("cargo:rustc-env=REACT_APP_DIR=app/dist");
}
