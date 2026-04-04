use archergate_license::LicenseClient;

#[tokio::main]
async fn main() {
    println!("Archergate Minimal Example");
    println!("==========================\n");

    // Create a client that connects to the validation server
    // In this example, we assume the server is running at http://localhost:3000
    let client = LicenseClient::new(
        "test-api-key",
        "com.archergate.example",
    );

    // Generate a test license key (in production, users provide their own)
    let test_key = "TEST-AAAA-BBBB-CCCC-DDDD";

    println!("Validating license key: {}", test_key);

    // Validate the key against the server
    match client.validate(test_key) {
        Ok(receipt) => {
            println!("SUCCESS: License is valid!");
            println!("  Machine fingerprint verified");
            println!("  Offline grace period: {} days", receipt.offline_days);
        }
        Err(e) => {
            eprintln!("FAIL: License validation failed");
            eprintln!("  Error: {}", e);
            eprintln!("\n  To test with a real key:");
            eprintln!("  1. Start the server: docker-compose up");
            eprintln!("  2. Create an API key and license via the curl commands in run.sh");
            eprintln!("  3. Update test_key in this file with the generated key");
            eprintln!("  4. Run: cargo run");
            std::process::exit(1);
        }
    }
}
