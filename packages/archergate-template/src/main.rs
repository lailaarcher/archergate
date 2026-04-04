use archergate_license::LicenseClient;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize license client
    let api_key = std::env::var("ARCHERGATE_API_KEY")
        .expect("Set ARCHERGATE_API_KEY environment variable");

    let client = LicenseClient::new(&api_key, "{{app_id}}");

    // Check for license key
    let license_key = std::env::var("LICENSE_KEY").unwrap_or_default();

    if !license_key.is_empty() {
        match client.validate(&license_key) {
            Ok(_) => println!("License valid."),
            Err(e) => {
                eprintln!("License error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        println!("No license key provided. Running in trial mode.");
    }

    // Your application starts here
    println!("{{app_name}} is running.");

    Ok(())
}
