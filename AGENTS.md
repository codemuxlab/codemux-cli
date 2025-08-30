# AGENTS.md - CodeMux Development Guide

## Essential Commands
- **Build**: `just build` (dev), `just release` (prod), `just capture` (capture binary only)
- **Test**: `just test` (all tests), `cargo test <test_name>` (single test), `just watch-test` (watch mode)
- **Lint**: `just lint-all` (all), `just clippy` (Rust), `cd app && npm run lint` (React)
- **Format**: `just fmt` (Rust), `cd app && npm run format` (React)
- **Setup**: `just setup` (full dev environment), `just app-install` (React deps only)

## Code Style - Rust
- **Imports**: Group by std, external crates, local modules with empty lines between
- **Types**: Use `anyhow::Result<T>` for error handling, `serde` derives for serialization
- **Naming**: snake_case for functions/variables, PascalCase for types/structs
- **Error Handling**: Use `anyhow` for application errors, return `Result<T>` from fallible functions

## Code Style - TypeScript/React
- **Formatting**: Use Biome with tabs, double quotes, organize imports automatically
- **Types**: Strict TypeScript, avoid `any`, use proper interfaces for API responses
- **Hooks**: TanStack Query for API state, Zustand for client state, proper dependency arrays
- **Naming**: camelCase for variables/functions, PascalCase for components, kebab-case for files
- **Error Handling**: Proper error boundaries, user-friendly error messages

## Required Pre-commit Checks
- Run `just lint-all` and fix all errors before committing
- Biome linting is MANDATORY for both app/ and website/ directories
- TypeScript bindings auto-generated via `just ts-bindings` when Rust structs change