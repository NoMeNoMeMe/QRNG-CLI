use clap::{self, Parser, Subcommand, ValueEnum};
use colored::*;
use inquire::Select;
use reqwest;
use serde::Deserialize;
use std::error::Error;
use std::io::{self};
use std::{thread, time}; // For shuffling, if needed later

#[derive(Parser)]
#[clap(name = "Quantum RNG CLI")]
#[clap(about = "Fetches quantum random numbers from ANU QRNG API")]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Lotto,

    RandomArray {
        #[clap(short, long)]
        data_type: Option<DataType>,

        #[clap(short, long)]
        length: Option<u16>,

        #[clap(short, long)]
        block_size: Option<u16>,
    },
}

#[derive(ValueEnum, Clone, Debug, PartialEq)]
enum DataType {
    Uint8,
    Uint16,
    Hex16,
}

#[derive(Deserialize)]
struct ApiResponse {
    data: Vec<u8>,
}

const API_URL: &str = "https://qrng.anu.edu.au/API/jsonI.php";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Lotto) => fetch_lotto_numbers().await?,
        Some(Commands::RandomArray {
            data_type,
            length,
            block_size,
        }) => fetch_random_array(data_type, length, block_size).await?,
        None => interactive_mode().await?,
    }

    // Wait for user input to exit the program
    wait_for_exit();

    Ok(())
}

// At the end of your main function or after processing the data
fn wait_for_exit() {
    println!("Press Enter to exit...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
}

async fn fetch_lotto_numbers() -> Result<(), Box<dyn Error>> {
    // Prompt the user to think about their intention
    println!("{}", "Focus on your intention...".bold().green());

    // Wait for 5 to 10 seconds
    let wait_time = time::Duration::from_secs(10);
    thread::sleep(wait_time);

    // Make the request to fetch 10 numbers (to ensure we have enough unique numbers)
    let url = format!("{API_URL}?length=10&type=uint8");
    let response = reqwest::get(&url).await?;

    // Check if the response status is not successful
    if !response.status().is_success() {
        if response.status().is_server_error() {
            println!("{}", "Error: Server issue, please try again later.".red());
        } else {
            println!(
                "{}",
                format!("Error: Failed to fetch data. Status: {}", response.status()).red()
            );
        }
        return Ok(()); // Return Ok to avoid panicking
    }

    // Parse the response body into the ApiResponse struct
    let body = response.text().await?;
    let api_response: Result<ApiResponse, serde_json::Error> = serde_json::from_str(&body);

    match api_response {
        Ok(response) => {
            // Map numbers to the range of 1 to 45
            let numbers: Vec<u8> = response.data.into_iter().map(|n| (n % 49) + 1).collect();

            // Ensure uniqueness by taking the first 6 unique numbers
            let unique_numbers: Vec<u8> = numbers
                .into_iter()
                .collect::<std::collections::HashSet<_>>()
                .into_iter()
                .take(6)
                .collect();

            // Sort the numbers
            let mut sorted_numbers = unique_numbers;
            sorted_numbers.sort(); // Sort the numbers in ascending order

            println!("Lotto Numbers: {:?}", sorted_numbers);
        }
        Err(_) => {
            // Handle the case where the API returns `{"success": false}` or missing fields
            if body.contains("\"success\":false") {
                println!("{}", "Error: Rate limit reached or other API issue.".red());
            } else {
                // If there is a decoding error, inform the user
                println!("{}", "Error: Failed to parse the response.".red());
            }
        }
    }

    Ok(())
}

#[derive(Deserialize)]
struct ApiResponseHex {
    data: Vec<String>,
}

async fn fetch_random_array(
    data_type: Option<DataType>,
    length: Option<u16>,
    block_size: Option<u16>,
) -> Result<(), Box<dyn Error>> {
    let data_type = data_type.unwrap_or_else(|| prompt_for_data_type());
    let length = length.unwrap_or_else(|| prompt_for_length());
    let block_size = if matches!(data_type, DataType::Hex16) {
        Some(block_size.unwrap_or_else(|| prompt_for_block_size()))
    } else {
        None
    };

    let data_type_str = data_type.to_lowercase(); // Convert enum variant to lowercase string

    let mut url = format!("{API_URL}?length={length}&type={}", data_type_str);
    if let Some(size) = block_size {
        url.push_str(&format!("&size={}", size));
    }

    // Make the request
    let response = reqwest::get(&url).await?;

    // Check if the response status is successful
    if !response.status().is_success() {
        return Err(format!("Failed to fetch data. Status: {}", response.status()).into());
    }

    // Parse the response body into the ApiResponse struct
    let body = response.text().await?;
    let api_response: Result<ApiResponseHex, serde_json::Error> = serde_json::from_str(&body);

    match api_response {
        Ok(response) => {
            // If the response is valid, print the data
            println!("Random Array: {:?}", response.data);
        }
        Err(_) => {
            // Handle the case where the API returns `{"success": false}` or missing fields
            if body.contains("\"success\":false") {
                println!("{}", "Error: Rate limit reached or other API issue.".red());
            } else {
                // If there is a decoding error, inform the user
                println!("{}", "Error: Failed to parse the response.".red());
            }
        }
    }

    Ok(())
}

impl DataType {
    fn to_lowercase(&self) -> String {
        match self {
            DataType::Uint8 => "uint8".to_string(),
            DataType::Uint16 => "uint16".to_string(),
            DataType::Hex16 => "hex16".to_string(),
        }
    }
}

fn prompt_for_data_type() -> DataType {
    let options = vec!["uint8", "uint16", "hex16"];
    let choice = Select::new("Choose data type:", options).prompt().unwrap();
    match choice {
        "uint8" => DataType::Uint8,
        "uint16" => DataType::Uint16,
        "hex16" => DataType::Hex16,
        _ => unreachable!(),
    }
}

fn prompt_for_length() -> u16 {
    println!("Enter array length (1-1024):");
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if let Ok(length) = input.trim().parse::<u16>() {
            if (1..=1024).contains(&length) {
                return length;
            }
        }
        println!("Invalid input. Enter a number between 1 and 1024.");
    }
}

fn prompt_for_block_size() -> u16 {
    println!("Enter block size (1-1024):");
    loop {
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        if let Ok(size) = input.trim().parse::<u16>() {
            if (1..=1024).contains(&size) {
                return size;
            }
        }
        println!("Invalid input. Enter a number between 1 and 1024.");
    }
}

async fn interactive_mode() -> Result<(), Box<dyn std::error::Error>> {
    let options = vec!["Lotto", "Random Array"];
    let choice = Select::new("Choose an option:", options).prompt()?;

    match choice {
        "Lotto" => {
            println!("{}", "Fetching Lotto numbers...".green());
            fetch_lotto_numbers().await?; // Use .await directly here
        }
        "Random Array" => {
            println!("{}", "Fetching Random Array...".blue());
            fetch_random_array(None, None, None).await?; // Use .await directly here
        }
        _ => println!("{}", "Invalid choice".red()),
    }

    Ok(())
}
