//! Download and compile Tree-sitter grammars.

use serde_json;
use std::fs;
use std::process::Command;
use tempfile;

use crate::grammar_registry;
use crate::runtime::Runtime;

/// Try to locate tree-sitter include directory containing parser.h
fn find_tree_sitter_include_path() -> Result<String, String> {
    // Try system include paths
    let system_paths = [
        "/usr/include",
        "/usr/local/include",
        "/opt/homebrew/include",
        "/usr/local/opt/tree-sitter/include",
    ];
    for path in system_paths {
        let header_path = std::path::Path::new(path).join("tree_sitter/parser.h");
        if header_path.exists() {
            return Ok(path.to_string());
        }
    }

    // Try to find via cargo metadata
    if let Ok(output) =
        std::process::Command::new("cargo").args(["metadata", "--format-version=1"]).output()
    {
        if output.status.success() {
            let metadata: serde_json::Value = serde_json::from_slice(&output.stdout)
                .map_err(|e| format!("Failed to parse cargo metadata: {}", e))?;
            if let Some(packages) =
                metadata.get("packages").and_then(|p: &serde_json::Value| p.as_array())
            {
                for package in packages {
                    if let Some(name) =
                        package.get("name").and_then(|n: &serde_json::Value| n.as_str())
                    {
                        if name == "tree-sitter" {
                            if let Some(manifest_path) = package
                                .get("manifest_path")
                                .and_then(|m: &serde_json::Value| m.as_str())
                            {
                                let manifest = std::path::Path::new(manifest_path);
                                if let Some(root) = manifest.parent() {
                                    let include_path = root.join("lib").join("include");
                                    if include_path.exists() {
                                        return Ok(include_path.to_string_lossy().to_string());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Try to find in target directory
    if let Ok(cargo_manifest_dir) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest_path = std::path::Path::new(&cargo_manifest_dir);
        // Look in target/build directory for tree-sitter-*/out/build
        let target_dir = manifest_path.join("../../target");
        if target_dir.exists() {
            // Use find command to locate parser.h
            if let Ok(output) = std::process::Command::new("find")
                .arg(&target_dir)
                .arg("-name")
                .arg("parser.h")
                .arg("-type")
                .arg("f")
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if line.contains("tree_sitter") {
                            let path: &std::path::Path = std::path::Path::new(line);
                            if let Some(parent) = path.parent() {
                                if let Some(grandparent) = parent.parent() {
                                    return Ok(grandparent.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Last resort: try to use pkg-config
    if let Ok(output) =
        std::process::Command::new("pkg-config").args(["--cflags", "tree-sitter"]).output()
    {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for part in stdout.split_whitespace() {
                if part.starts_with("-I") {
                    let path = &part[2..];
                    return Ok(path.to_string());
                }
            }
        }
    }

    Err("Could not find tree-sitter include path".to_string())
}

/// Build a Tree-sitter grammar and install it to the runtime directory
pub fn build_and_install_grammar(language_id: &str) -> Result<(), String> {
    let grammar_info = grammar_registry::for_language(language_id)
        .ok_or_else(|| format!("No grammar info available for {}", language_id))?;

    // Create temporary directory
    let temp_dir =
        tempfile::tempdir().map_err(|e| format!("Failed to create temp directory: {}", e))?;

    // Download source as zip instead of using git clone
    println!("Downloading {} grammar source...", language_id);
    let repo_dir = temp_dir.path().join("repo");

    // Create repo directory
    fs::create_dir_all(&repo_dir)
        .map_err(|e| format!("Failed to create directory {}: {}", repo_dir.display(), e))?;

    // Special handling for markdown to ensure correct URL and structure
    let (repo_url, subdirectory) = if language_id == "markdown" {
        println!("Using corrected markdown repository URL and structure...");
        (
            "https://github.com/tree-sitter-grammars/tree-sitter-markdown".to_string(),
            Some("tree-sitter-markdown-inline".to_string()),
        )
    } else {
        (grammar_info.repo_url.clone(), grammar_info.subdirectory.clone())
    };

    println!("Cloning {}...", repo_url);

    // Clone repository with git
    println!("Cloning {}...", repo_url);

    let mut cmd = Command::new("git");
    cmd.args(["clone", "--depth", "1"]);
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd.args([&repo_url, repo_dir.to_str().unwrap()]);

    let status = cmd.status().map_err(|e| format!("Failed to run git clone: {}", e))?;

    if !status.success() {
        return Err(format!("Failed to clone repository. Exit code: {:?}", status.code()));
    }

    println!("Successfully cloned repository");

    // No zip extraction needed - we cloned directly into repo_dir

    // We cloned directly into repo_dir, so source_dir is repo_dir
    // Navigate to subdirectory if needed
    let source_dir =
        if let Some(subdir) = &subdirectory { repo_dir.join(subdir) } else { repo_dir.clone() };

    // Verify source directory exists
    if !source_dir.exists() {
        return Err(format!("Source directory does not exist: {:?}", source_dir));
    }

    // For languages with subdirectories, we need to check if the source files exist
    // relative to the source_dir, not the repo root
    // The source_files in grammar_info are relative to the subdirectory
    // So we don't need to adjust them

    // Check if tree-sitter CLI is available and at a compatible version
    let has_tree_sitter_cli =
        Command::new("tree-sitter").arg("--version").output().is_ok_and(|output| {
            if output.status.success() {
                let version_str = String::from_utf8_lossy(&output.stdout);
                // Check if version is at least 0.20.0
                version_str.contains("0.20")
                    || version_str.contains("0.21")
                    || version_str.contains("0.22")
                    || version_str.contains("0.23")
                    || version_str.contains("0.24")
                    || version_str.contains("0.25")
                    || version_str.contains("0.26")
            } else {
                false
            }
        });

    let lib_path;

    if has_tree_sitter_cli {
        // Use tree-sitter CLI
        println!("Using tree-sitter CLI to build {}...", language_id);

        // For TypeScript/TSX, the grammar.json is in the source_dir itself, not in a further src subdirectory
        // Determine the directory to run tree-sitter build in
        let build_dir: &std::path::Path = if repo_dir.join("grammar.js").exists()
            || repo_dir.join("grammar.json").exists()
        {
            &repo_dir
        } else if source_dir.join("grammar.js").exists() || source_dir.join("grammar.json").exists()
        {
            &source_dir
        } else {
            // For TypeScript/TSX, check if grammar.json exists in the parent directory
            // This handles the case where source_dir is "typescript/src" but grammar.json is in "typescript/"
            if language_id == "typescript" || language_id == "tsx" {
                if let Some(parent) = source_dir.parent() {
                    if parent.join("grammar.js").exists() || parent.join("grammar.json").exists() {
                        parent
                    } else {
                        &source_dir
                    }
                } else {
                    &source_dir
                }
            } else {
                &source_dir
            }
        };

        // Check if package.json exists and install dependencies if needed
        if build_dir.join("package.json").exists() {
            println!("Installing npm dependencies...");
            let install_output = Command::new("npm")
                .current_dir(build_dir)
                .arg("install")
                .output()
                .map_err(|e| format!("Failed to run npm install: {}", e))?;

            if !install_output.status.success() {
                let stderr = String::from_utf8_lossy(&install_output.stderr);
                let stdout = String::from_utf8_lossy(&install_output.stdout);
                eprintln!("npm install output:\nstdout: {}\nstderr: {}", stdout, stderr);
                // Continue anyway, maybe dependencies are already installed
            } else {
                println!("npm dependencies installed successfully");
            }
        }

        // Run tree-sitter generate if needed
        // Check if parser.c exists in the source directory
        let parser_c_exists =
            source_dir.join("parser.c").exists() || source_dir.join("src/parser.c").exists();

        if !parser_c_exists {
            // Check if grammar.js or grammar.json exists in build_dir
            let grammar_js_exists = build_dir.join("grammar.js").exists();
            let grammar_json_exists = build_dir.join("grammar.json").exists();

            if grammar_js_exists || grammar_json_exists {
                println!("Running tree-sitter generate in {}...", build_dir.display());
                let generate_output = Command::new("tree-sitter")
                    .current_dir(build_dir)
                    .arg("generate")
                    .output()
                    .map_err(|e| format!("Failed to run tree-sitter generate: {}", e))?;

                if !generate_output.status.success() {
                    let stderr = String::from_utf8_lossy(&generate_output.stderr);
                    let stdout = String::from_utf8_lossy(&generate_output.stdout);
                    eprintln!(
                        "tree-sitter generate failed:\nstdout: {}\nstderr: {}",
                        stdout, stderr
                    );
                    // Continue anyway, maybe parser.c already exists elsewhere
                } else {
                    println!("tree-sitter generate succeeded");
                }
            } else {
                println!("No grammar.js or grammar.json found, skipping tree-sitter generate");
            }
        } else {
            println!("parser.c already exists, skipping tree-sitter generate");
        }

        // Always run tree-sitter build when CLI is available
        println!("Running tree-sitter build for {} in {}...", language_id, build_dir.display());

        // For TypeScript/TSX, we need to run tree-sitter build in the subdirectory
        let mut cmd = Command::new("tree-sitter");
        cmd.current_dir(build_dir);

        // Always use "build" command without --grammar flag
        cmd.arg("build");

        let build_output =
            cmd.output().map_err(|e| format!("Failed to run tree-sitter build: {}", e))?;

        if !build_output.status.success() {
            let stderr = String::from_utf8_lossy(&build_output.stderr);
            let stdout = String::from_utf8_lossy(&build_output.stdout);
            eprintln!("tree-sitter build failed:\nstdout: {}\nstderr: {}", stdout, stderr);

            // Fall back to manual compilation
            println!("tree-sitter build failed, falling back to manual compilation...");
            let lib_path =
                manual_compile(&grammar_info, &source_dir, &repo_dir, language_id, &temp_dir)?;
            return install_library_and_queries(
                &grammar_info,
                &source_dir,
                &repo_dir,
                language_id,
                &temp_dir,
                lib_path,
            );
        }

        // Print build output for debugging
        let build_stdout = String::from_utf8_lossy(&build_output.stdout);
        let build_stderr = String::from_utf8_lossy(&build_output.stderr);
        if !build_stdout.trim().is_empty() {
            println!("tree-sitter build stdout: {}", build_stdout);
        }
        if !build_stderr.trim().is_empty() {
            println!("tree-sitter build stderr: {}", build_stderr);
        }

        // Find the built library
        let lib_name = get_library_name(language_id);
        let parser_lib_name = if cfg!(windows) {
            "parser.dll"
        } else if cfg!(target_os = "macos") {
            "parser.dylib"
        } else {
            "parser.so"
        };

        // Check common locations
        let possible_paths = vec![
            source_dir.join(parser_lib_name),
            source_dir.join(&lib_name),
            source_dir.join("target").join("release").join(parser_lib_name),
            source_dir.join("target").join("release").join(&lib_name),
            build_dir.join(parser_lib_name),
            build_dir.join(&lib_name),
            build_dir.join("target").join("release").join(parser_lib_name),
            build_dir.join("target").join("release").join(&lib_name),
        ];

        // For markdown, also check markdown-inline.so
        let markdown_paths = if language_id == "markdown" {
            let markdown_lib_name = if cfg!(windows) {
                "markdown-inline.dll"
            } else if cfg!(target_os = "macos") {
                "markdown-inline.dylib"
            } else {
                "markdown-inline.so"
            };
            vec![
                source_dir.join(markdown_lib_name),
                source_dir.join("target").join("release").join(markdown_lib_name),
                build_dir.join(markdown_lib_name),
                build_dir.join("target").join("release").join(markdown_lib_name),
            ]
        } else {
            vec![]
        };

        let all_paths: Vec<std::path::PathBuf> =
            possible_paths.into_iter().chain(markdown_paths.into_iter()).collect();

        let mut found = None;
        for path in &all_paths {
            if path.exists() {
                found = Some(path.clone());
                break;
            }
        }

        lib_path =
            found.ok_or_else(|| format!("Could not find built library after tree-sitter build"))?;
    } else {
        // Manual compilation with cc
        println!("Using cc to build {}...", language_id);
        let lib_path =
            manual_compile(&grammar_info, &source_dir, &repo_dir, language_id, &temp_dir)?;
        return install_library_and_queries(
            &grammar_info,
            &source_dir,
            &repo_dir,
            language_id,
            &temp_dir,
            lib_path,
        );
    }

    // Use the helper function to install library and queries
    install_library_and_queries(
        &grammar_info,
        &source_dir,
        &repo_dir,
        language_id,
        &temp_dir,
        lib_path,
    )
}

/// Generate default language.toml content for a language
fn generate_language_toml(
    language_id: &str,
    grammar_info: &crate::grammar_registry::GrammarInfo,
) -> String {
    // Determine comment syntax based on language
    let (comment_line, comment_start, comment_end) = match language_id {
        "rust" | "c" | "cpp" | "c_sharp" | "java" | "javascript" | "typescript" | "tsx" | "go" => {
            ("//", "/*", "*/")
        }
        "python" | "bash" | "ruby" | "yaml" | "dockerfile" | "cmake" | "toml" => ("#", "", ""),
        "lua" => ("--", "--[[", "]]"),
        "html" | "css" => ("", "<!--", "-->"),
        "markdown" => ("", "", ""),
        _ => ("//", "/*", "*/"), // Default
    };

    // Basic template with common zaroxi_infra_settings
    // This can be expanded based on language-specific needs
    format!(
        r#"[language]
id = "{}"
name = "{}"
extensions = [{}]
comment_line = "{}"
comment_start = "{}"
comment_end = "{}"

[brackets]
pairs = [
    ["(", ")"],
    ["[", "]"],
    ["{{", "}}"],
    ["\"", "\""],
    ["'", "'"],
]

[indentation]
width = 4
use_tabs = false
"#,
        language_id,
        grammar_info.name,
        grammar_info
            .extensions
            .iter()
            .map(|ext| format!("\"{}\"", ext))
            .collect::<Vec<_>>()
            .join(", "),
        comment_line,
        comment_start,
        comment_end
    )
}

/// Install library and queries after compilation
fn install_library_and_queries(
    grammar_info: &crate::grammar_registry::GrammarInfo,
    source_dir: &std::path::Path,
    repo_dir: &std::path::Path,
    language_id: &str,
    temp_dir: &tempfile::TempDir,
    lib_path: std::path::PathBuf,
) -> Result<(), String> {
    let runtime = Runtime::new();

    // Install library to runtime directory
    let target_dir = runtime.grammar_dir();
    fs::create_dir_all(&target_dir)
        .map_err(|e| format!("Failed to create target directory: {}", e))?;

    let target_lib_path = target_dir.join(get_library_name(language_id));

    // For markdown, if the built library has a different name, rename it
    let source_lib_path = if language_id == "markdown" {
        let has_inline_name = lib_path
            .file_name()
            .and_then(|n: &std::ffi::OsStr| n.to_str())
            .map(|n: &str| n.contains("markdown-inline"))
            .unwrap_or(false);
        if has_inline_name {
            // Rename markdown-inline.so to libtree-sitter-markdown.so
            let renamed_path = lib_path.parent().unwrap().join(get_library_name(language_id));
            fs::copy(&lib_path, &renamed_path)
                .map_err(|e| format!("Failed to rename library: {}", e))?;
            renamed_path
        } else {
            lib_path
        }
    } else {
        lib_path
    };

    fs::copy(&source_lib_path, &target_lib_path)
        .map_err(|e| format!("Failed to copy library: {}", e))?;

    println!("Installed library to: {}", target_lib_path.display());

    // Install query files
    // For markdown, queries are in the parent directory (tree-sitter-markdown/queries)
    // But the markdown grammar uses tree-sitter-markdown-inline subdirectory
    // So we need to look in multiple possible locations
    let query_source_dir = if language_id == "markdown" {
        // Try multiple possible locations
        let possible_dirs = vec![
            // Look in the parent directory of source_dir (tree-sitter-markdown/queries)
            source_dir.parent().unwrap_or(repo_dir).join("queries"),
            // Look in the repo root's queries directory
            repo_dir.join("queries"),
            // Look in source_dir/queries (unlikely but possible)
            source_dir.join("queries"),
        ];

        // Find the first existing directory
        let mut found_dir = None;
        for dir in possible_dirs {
            if dir.exists() {
                println!("Found markdown queries at: {}", dir.display());
                found_dir = Some(dir);
                break;
            }
        }
        // If none found, use the parent directory
        found_dir.unwrap_or_else(|| source_dir.parent().unwrap_or(repo_dir).join("queries"))
    } else {
        // For all languages, try multiple possible query locations
        let mut possible_dirs = Vec::new();

        // 1. Check source_dir/queries (most common)
        possible_dirs.push(source_dir.join("queries"));

        // 2. Check parent directory queries (for languages in subdirectories)
        if let Some(parent) = source_dir.parent() {
            possible_dirs.push(parent.join("queries"));
        }

        // 3. Check repo root queries
        possible_dirs.push(repo_dir.join("queries"));

        // 4. For all languages, check language-specific query subdirectories
        // Many grammars store queries in queries/{language_id}/ directory
        if let Some(parent) = source_dir.parent() {
            let repo_queries_dir = parent.join("queries");
            if repo_queries_dir.exists() {
                // Check for language-specific subdirectory
                let language_queries_dir = repo_queries_dir.join(language_id);
                if language_queries_dir.exists() {
                    possible_dirs.push(language_queries_dir);
                }
                // Also add the root queries directory (already added above, but keep for clarity)
            }
        }

        // 5. Also check for queries in the source directory itself (some grammars have queries in src/queries)
        let source_queries_dir = source_dir.join("queries");
        if source_queries_dir.exists() {
            possible_dirs.push(source_queries_dir);
        }

        // 6. For TypeScript/TSX, also check the specific structure
        if language_id == "typescript" || language_id == "tsx" {
            // In tree-sitter-typescript, queries might be in the language subdirectory directly
            possible_dirs.push(source_dir.to_path_buf());
            // Or in a sibling queries directory
            if let Some(parent) = source_dir.parent() {
                possible_dirs.push(parent.join("queries"));
                // Also check for queries/typescript or queries/tsx in the repo root
                let repo_queries_subdir = parent.join("queries").join(language_id);
                possible_dirs.push(repo_queries_subdir);
            }
        }

        // Find the first existing directory
        let mut found_dir = None;
        for dir in &possible_dirs {
            if dir.exists() {
                println!("Found query directory for {} at: {}", language_id, dir.display());
                found_dir = Some(dir.clone());
                break;
            }
        }

        // If none found, use source_dir/queries as default (even if it doesn't exist)
        found_dir.unwrap_or_else(|| source_dir.join("queries"))
    };

    // Always create the query target directory
    let query_target_dir = runtime.language_dir(language_id).join("queries");
    fs::create_dir_all(&query_target_dir)
        .map_err(|e| format!("Failed to create query directory: {}", e))?;

    // Collect all potential query source directories to check
    let mut potential_dirs = Vec::new();

    // Add the primary query source directory
    if query_source_dir.exists() {
        potential_dirs.push(query_source_dir.clone());
    }

    // Add the repo root queries directory
    let repo_queries_dir = temp_dir.path().join("repo").join("queries");
    if repo_queries_dir.exists() {
        potential_dirs.push(repo_queries_dir);
    }

    // For languages with subdirectories, also check the parent directory's queries
    if let Some(_subdir) = &grammar_info.subdirectory {
        let parent_dir = temp_dir.path().join("repo");
        let parent_queries = parent_dir.join("queries");
        if parent_queries.exists() && !potential_dirs.contains(&parent_queries) {
            potential_dirs.push(parent_queries);
        }
    }

    // Try to copy each query file from any potential directory
    for query_file in &grammar_info.query_files {
        let mut copied = false;

        for source_dir in &potential_dirs {
            let source_path = source_dir.join(query_file);
            if source_path.exists() {
                let target_path = query_target_dir.join(query_file);
                match fs::copy(&source_path, &target_path) {
                    Ok(_) => {
                        println!(
                            "Installed query file: {} from {}",
                            query_file,
                            source_dir.display()
                        );
                        copied = true;
                        break;
                    }
                    Err(e) => {
                        println!("Warning: Failed to copy query file {}: {}", query_file, e);
                    }
                }
            }
        }

        if !copied {
            println!(
                "Note: Query file {} not found for {} (this may be normal if the grammar doesn't provide it)",
                query_file, language_id
            );
        }
    }

    // Create language.toml if it doesn't exist
    let language_toml_path = query_target_dir.join("language.toml");
    if !language_toml_path.exists() {
        println!("Creating default language.toml for {}...", language_id);
        let toml_content = generate_language_toml(language_id, grammar_info);
        fs::write(&language_toml_path, toml_content)
            .map_err(|e| format!("Failed to write language.toml: {}", e))?;
        println!("Created language.toml at: {}", language_toml_path.display());
    } else {
        println!("language.toml already exists for {}", language_id);
    }

    println!("Successfully installed {} grammar!", language_id);
    Ok(())
}

/// Manual compilation fallback when tree-sitter CLI fails or is unavailable
fn manual_compile(
    grammar_info: &crate::grammar_registry::GrammarInfo,
    source_dir: &std::path::Path,
    _repo_dir: &std::path::Path,
    language_id: &str,
    temp_dir: &tempfile::TempDir,
) -> Result<std::path::PathBuf, String> {
    println!("Compiling {} manually with cc...", language_id);

    // Check if source files exist
    let mut source_files_exist = true;
    for source_file in &grammar_info.source_files {
        if !source_dir.join(source_file).exists() {
            println!("Warning: Source file {} does not exist", source_file);
            source_files_exist = false;
        }
    }

    if !source_files_exist {
        return Err(format!("Some source files are missing for {}", language_id));
    }

    // Compile all source files
    let mut object_files = Vec::new();
    for source_file in &grammar_info.source_files {
        let source_path = source_dir.join(source_file);
        if !source_path.exists() {
            println!("Warning: Source file {} does not exist, skipping", source_file);
            continue; // Skip missing files (some grammars don't have scanner.c)
        }

        let object_file = temp_dir.path().join(format!("{}.o", source_file.replace('/', "_")));

        println!("Compiling {}...", source_file);
        // Try to find tree-sitter include path
        let mut include_args = vec!["-c", "-fPIC"];

        // Add include path for the source directory
        include_args.push("-I");
        include_args.push(source_dir.to_str().unwrap());

        // For TypeScript/TSX, we need to include the common directory which is two levels up
        if language_id == "typescript" || language_id == "tsx" {
            if let Some(parent) = source_dir.parent() {
                if let Some(grandparent) = parent.parent() {
                    let common_dir = grandparent.join("common");
                    if common_dir.exists() {
                        include_args.push("-I");
                        include_args.push(grandparent.to_str().unwrap());
                    }
                }
            }
        }

        // Store tree-sitter include path in a variable that lives long enough
        let tree_sitter_include = find_tree_sitter_include_path().ok();

        // Add include path for tree-sitter headers if available
        if let Some(tree_sitter_include) = &tree_sitter_include {
            include_args.push("-I");
            include_args.push(tree_sitter_include);
        }

        // Add include path for the repo root (for common/ directory)
        if let Some(repo_root) = source_dir.parent() {
            if repo_root.join("common").exists() {
                include_args.push("-I");
                include_args.push(repo_root.to_str().unwrap());
            }
        }

        include_args.extend_from_slice(&["-o", object_file.to_str().unwrap()]);

        let output = std::process::Command::new("cc")
            .args(&include_args)
            .arg(&source_path)
            .current_dir(&source_dir)
            .output()
            .map_err(|e| format!("Failed to compile {}: {}", source_file, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("Failed to compile {}: {}", source_file, stderr));
        }

        object_files.push(object_file);
    }

    if object_files.is_empty() {
        return Err("No source files compiled".to_string());
    }

    // Link shared library
    let lib_name = get_library_name(language_id);
    let lib_path = temp_dir.path().join(&lib_name);

    let mut cmd = std::process::Command::new("cc");
    cmd.args(["-shared", "-fPIC", "-o"]).arg(&lib_path);

    for obj in &object_files {
        cmd.arg(obj);
    }

    cmd.arg("-lstdc++");

    let status = cmd.status().map_err(|e| format!("Failed to link library: {}", e))?;

    if !status.success() {
        return Err("Failed to link shared library".to_string());
    }

    Ok(lib_path)
}

/// Get the platform-specific library name for a language
fn get_library_name(language_id: &str) -> String {
    let prefix = if cfg!(windows) { "" } else { "lib" };
    let extension = if cfg!(windows) {
        ".dll"
    } else if cfg!(target_os = "macos") {
        ".dylib"
    } else {
        ".so"
    };

    // Some language IDs use underscores but the library uses hyphens
    // For example: "c_sharp" -> "c-sharp" in library name
    let lib_name = match language_id {
        "c_sharp" => "c-sharp",
        _ => language_id,
    };

    format!("{}tree-sitter-{}{}", prefix, lib_name, extension)
}

/// Check if a grammar is installed
pub fn is_grammar_installed(language_id: &str) -> bool {
    let runtime = Runtime::new();
    let lib_path = runtime.grammar_library_path(language_id);
    lib_path.exists()
}

/// Install missing grammars for a list of languages
pub fn install_missing_grammars(language_ids: &[&str]) -> Vec<String> {
    let mut installed = Vec::new();

    for &language_id in language_ids {
        if !is_grammar_installed(language_id) {
            println!("Grammar for {} is not installed. Installing...", language_id);
            match build_and_install_grammar(language_id) {
                Ok(()) => {
                    installed.push(language_id.to_string());
                    println!("Successfully installed {} grammar", language_id);
                }
                Err(e) => {
                    eprintln!("Failed to install {} grammar: {}", language_id, e);
                    eprintln!("Will continue with other grammars...");
                }
            }
        } else {
            println!("Grammar for {} is already installed", language_id);
        }
    }

    installed
}
