use std::fs;
use std::io::Write;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace_toml = fs::read_to_string("Cargo.toml")
        .map_err(|e| format!("failed to read workspace Cargo.toml: {}", e))?;

    // Naive parse to extract members = [ ... ] block.
    let members_start = workspace_toml
        .find("members")
        .and_then(|i| workspace_toml[i..].find('[').map(|j| i + j))
        .ok_or("failed to find members [ in Cargo.toml")?;

    let mut depth = 0isize;
    let mut members_block = String::new();
    for c in workspace_toml[members_start..].chars() {
        if c == '[' {
            depth += 1;
            // skip the opening bracket itself
            if depth == 1 {
                continue;
            }
        }
        if c == ']' {
            depth -= 1;
            if depth == 0 {
                break;
            }
        }
        if depth >= 1 {
            members_block.push(c);
        }
    }

    if members_block.trim().is_empty() {
        return Err("no members found in Cargo.toml".into());
    }

    let mut created = 0usize;

    for line in members_block.lines() {
        let s = line.trim();
        if s.is_empty() {
            continue;
        }
        // Remove trailing commas and surrounding quotes
        let s = s.trim_end_matches(',');
        let s = s.trim_matches('"').trim_matches('\'').trim();
        if s.is_empty() {
            continue;
        }

        // Skip docs path entries or tools that are not crate dirs (we will still create them)
        let path = Path::new(s);

        // Create directory if missing
        if !path.exists() {
            fs::create_dir_all(path.join("src"))?;
            // Write Cargo.toml for the crate
            let crate_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .ok_or("invalid crate path")?;

            let cargo_toml = format!(
                r#"[package]
name = "{crate_name}"
version = "0.1.0"
edition = "2024"
license = "MIT"
"#,
                crate_name = crate_name
            );

            fs::write(path.join("Cargo.toml"), cargo_toml)?;

            // Write src/lib.rs
            let lib_rs = format!(
                r#"//! Auto-generated stub for `{crate_name}`.
//! Replace with real crate contents.

/// The crate name (auto-generated).
pub const CRATE_NAME: &str = "{crate_name}";

/// Basic info helper.
pub fn info() -> &'static str {{
    CRATE_NAME
}}
"#,
                crate_name = crate_name
            );

            let mut f = fs::File::create(path.join("src").join("lib.rs"))?;
            f.write_all(lib_rs.as_bytes())?;

            println!("created stub for: {}", s);
            created += 1;
        } else {
            println!("skipped existing path: {}", s);
        }
    }

    println!("done. created {} crate stubs.", created);
    Ok(())
}
 use std::fs;
 use std::io::Write;
 use std::path::Path;

 fn main() -> Result<(), Box<dyn std::error::Error>> {
     let workspace_toml = fs::read_to_string("Cargo.toml")
         .map_err(|e| format!("failed to read workspace Cargo.toml: {}", e))?;

     // Naive parse to extract members = [ ... ] block.
     let members_start = workspace_toml
         .find("members")
         .and_then(|i| workspace_toml[i..].find('[').map(|j| i + j))
         .ok_or("failed to find members [ in Cargo.toml")?;

     let mut depth = 0isize;
     let mut members_block = String::new();
     for c in workspace_toml[members_start..].chars() {
         if c == '[' {
             depth += 1;
             // skip the opening bracket itself
             if depth == 1 {
                 continue;
             }
         }
         if c == ']' {
             depth -= 1;
             if depth == 0 {
                 break;
             }
         }
         if depth >= 1 {
             members_block.push(c);
         }
     }

     if members_block.trim().is_empty() {
         return Err("no members found in Cargo.toml".into());
     }

     let mut created = 0usize;

     for line in members_block.lines() {
         let s = line.trim();
         if s.is_empty() {
             continue;
         }
         // Remove trailing commas and surrounding quotes
         let s = s.trim_end_matches(',');
         let s = s.trim_matches('"').trim_matches('\'').trim();
         if s.is_empty() {
             continue;
         }

         // Skip docs path entries or tools that are not crate dirs (we will still create them)
         let path = Path::new(s);

         // Create directory if missing
         if !path.exists() {
             fs::create_dir_all(path.join("src"))?;
             // Write Cargo.toml for the crate
             let crate_name = path
                 .file_name()
                 .and_then(|n| n.to_str())
                 .ok_or("invalid crate path")?;

             let cargo_toml = format!(
                 r#"[package]
 name = "{crate_name}"
 version = "0.1.0"
 edition = "2024"
 license = "MIT"
 "#,
                 crate_name = crate_name
             );

             fs::write(path.join("Cargo.toml"), cargo_toml)?;

             // Write src/lib.rs
             let lib_rs = format!(
                 r#"//! Auto-generated stub for `{crate_name}`.
 //! Replace with real crate contents.

 /// The crate name (auto-generated).
 pub const CRATE_NAME: &str = "{crate_name}";

 /// Basic info helper.
 pub fn info() -> &'static str {{
     CRATE_NAME
 }}
 "#,
                 crate_name = crate_name
             );

             let mut f = fs::File::create(path.join("src").join("lib.rs"))?;
             f.write_all(lib_rs.as_bytes())?;

             println!("created stub for: {}", s);
             created += 1;
         } else {
             println!("skipped existing path: {}", s);
         }
     }

     println!("done. created {} crate stubs.", created);
     Ok(())
 }
