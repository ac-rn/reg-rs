//! Comparison tests for benchmarking and validation against other parsers.
//!
//! This test suite collects metrics from our Rust parser that can be compared
//! against other implementations (e.g., regipy).

use reg_parser::{Hive, RegistryKey};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

fn test_data_path(filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("test_data")
        .join(filename)
}

fn output_path(filename: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("comparison")
        .join("results")
        .join(filename)
}

#[derive(Debug, Serialize, Deserialize)]
struct SampleValue {
    path: String,
    name: String,
    data_hex: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ComparisonResult {
    parser: String,
    file: String,
    key_count: usize,
    value_count: usize,
    parse_time_ms: f64,
    sample_values: Vec<SampleValue>,
}

/// Recursively count keys and values in a registry hive
fn count_keys_and_values(
    key: &RegistryKey,
    key_count: &mut usize,
    value_count: &mut usize,
    sample_values: &mut Vec<SampleValue>,
    current_path: &str,
    max_samples: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    *key_count += 1;

    // Count and sample values
    if let Ok(values) = key.values() {
        *value_count += values.len();

        // Collect sample values (first few from each key)
        if sample_values.len() < max_samples {
            for value in values.iter().take(3) {
                if sample_values.len() >= max_samples {
                    break;
                }

                // Get raw data and hex encode it
                if let Ok(raw_data) = value.raw_data() {
                    let data_hex = hex::encode(&raw_data);
                    sample_values.push(SampleValue {
                        path: current_path.to_string(),
                        name: value.name().to_string(),
                        data_hex,
                    });
                }
            }
        }
    }

    // Recurse into subkeys
    if let Ok(subkeys) = key.subkeys() {
        for subkey in subkeys {
            let subkey_name = subkey.name().unwrap_or_else(|_| "(unnamed)".to_string());
            let subkey_path = if current_path.is_empty() {
                subkey_name.clone()
            } else {
                format!("{}\\{}", current_path, subkey_name)
            };

            count_keys_and_values(
                &subkey,
                key_count,
                value_count,
                sample_values,
                &subkey_path,
                max_samples,
            )?;
        }
    }

    Ok(())
}

fn analyze_hive(hive_path: &PathBuf, hive_name: &str) -> Result<ComparisonResult, Box<dyn std::error::Error>> {
    println!("\n=== Analyzing {} with Rust parser ===", hive_name);

    let start = Instant::now();
    let hive = Hive::open(hive_path)?;
    let root = hive.root_key()?;

    let mut key_count = 0;
    let mut value_count = 0;
    let mut sample_values = Vec::new();

    count_keys_and_values(
        &root,
        &mut key_count,
        &mut value_count,
        &mut sample_values,
        "",
        100, // Collect up to 100 sample values
    )?;

    let elapsed = start.elapsed();
    let parse_time_ms = elapsed.as_secs_f64() * 1000.0;

    println!("Keys: {}", key_count);
    println!("Values: {}", value_count);
    println!("Parse time: {:.2} ms", parse_time_ms);
    println!("Sample values collected: {}", sample_values.len());

    Ok(ComparisonResult {
        parser: "rust".to_string(),
        file: hive_name.to_string(),
        key_count,
        value_count,
        parse_time_ms,
        sample_values,
    })
}

fn save_result(result: &ComparisonResult) -> Result<(), Box<dyn std::error::Error>> {
    // Create results directory if it doesn't exist
    let results_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("comparison")
        .join("results");
    std::fs::create_dir_all(&results_dir)?;

    let filename = format!("rust_{}.json", result.file.to_lowercase().replace('.', "_"));
    let output = output_path(&filename);

    let json = serde_json::to_string_pretty(&result)?;
    let mut file = File::create(&output)?;
    file.write_all(json.as_bytes())?;

    println!("Results saved to: {}", output.display());

    Ok(())
}

#[test]
fn test_comparison_ntuser_dat() {
    let path = test_data_path("NTUSER.DAT");
    
    if !path.exists() {
        println!("Skipping test: NTUSER.DAT not found at {:?}", path);
        return;
    }

    let result = analyze_hive(&path, "NTUSER.DAT")
        .expect("Failed to analyze NTUSER.DAT");

    save_result(&result).expect("Failed to save results");

    // Basic sanity checks
    assert!(result.key_count > 0, "Should have found some keys");
    assert!(result.parse_time_ms > 0.0, "Parse time should be positive");
}

#[test]
fn test_comparison_usrclass_dat() {
    let path = test_data_path("UsrClass.dat");
    
    if !path.exists() {
        println!("Skipping test: UsrClass.dat not found at {:?}", path);
        return;
    }

    let result = analyze_hive(&path, "UsrClass.dat")
        .expect("Failed to analyze UsrClass.dat");

    save_result(&result).expect("Failed to save results");

    // Basic sanity checks
    assert!(result.key_count > 0, "Should have found some keys");
    assert!(result.parse_time_ms > 0.0, "Parse time should be positive");
}

#[test]
fn test_comparison_all_hives() {
    let hives = vec![
        ("NTUSER.DAT", "NTUSER.DAT"),
        ("UsrClass.dat", "UsrClass.dat"),
    ];

    for (filename, display_name) in hives {
        let path = test_data_path(filename);
        
        if !path.exists() {
            println!("Skipping {}: file not found", display_name);
            continue;
        }

        match analyze_hive(&path, display_name) {
            Ok(result) => {
                if let Err(e) = save_result(&result) {
                    eprintln!("Failed to save results for {}: {}", display_name, e);
                }
            }
            Err(e) => {
                eprintln!("Failed to analyze {}: {}", display_name, e);
            }
        }
    }
}
