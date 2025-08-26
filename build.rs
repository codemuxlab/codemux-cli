use std::process::Command;
use std::path::Path;
use std::env;

fn main() {
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
        
        let npm_install = Command::new("npm")
            .args(&["install"])
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
    let output = Command::new("npm")
        .args(&["run", "build"])
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