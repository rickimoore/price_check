use rodio::{Decoder, OutputStream, Sink};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use tokio::time::{self, Duration};
use clap::Parser;

#[derive(Parser)]
struct Args {
    #[clap(short, long, default_value= "BTC")]
    asset: String,
    #[clap(short, long, default_value= "USDC")]
    currency: String,
    #[clap(short, long, default_value= "100.0")]
    diff: f64,
}

#[derive(Deserialize, Serialize, Debug)]
struct ResponseData {
    currency: String,
    rates: HashMap<String, String>,
}

#[derive(Deserialize, Serialize, Debug)]
struct Response {
    data: ResponseData,
}

fn play_jingle(path: &str) {
    // Get a handle to the default audio output device
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = Sink::try_new(&stream_handle).unwrap();

    // Open the MP3 file
    let file = BufReader::new(File::open(path).unwrap());

    // Decode the MP3 file
    let source = Decoder::new_mp3(file).unwrap();

    // Play the decoded MP3 file
    sink.append(source);

    // Block the current thread until the sink has finished playing all its queued sounds
    sink.sleep_until_end();
}

#[tokio::main]
async fn main() {
    let mut initial_rate: f64;
    let mut lower_low: f64;

    let cli = Args::parse();


    let asset: String = cli.asset.to_uppercase();
    let currency: String = cli.currency.to_uppercase();
    let difference: f64 = cli.diff;

    println!("[ {} ] asset accepted...", asset);
    println!("[ {} ] currency rate accepted...", currency);
    println!("Monitoring for price change of +- [ {} ]", difference);

    println!("Contacting coinbase api...");
    match fetch_rate(&asset, &currency).await {
        Ok(rate) => {
            println!("Successful api response...");
            initial_rate = rate;
            lower_low = rate;
            println!(
                "Setting initial exchange rate {} to USD: {}",
                asset, initial_rate
            );
        }
        Err(e) => {
            eprintln!("Error fetching rate: {}", e);
            std::process::exit(-1);
        }
    }

    let mut interval = time::interval(Duration::from_secs(10));

    loop {
        interval.tick().await;
        match fetch_rate(&asset, &currency).await {
            Ok(rate) => {
                println!("Comparing new rate {} to USD: {}", asset, rate);
                if (rate - initial_rate) > difference {
                    initial_rate = rate;
                    lower_low = rate;
                    println!(
                        "Setting new initial exchange rate {} to USD: {}",
                        asset, initial_rate
                    );
                    play_jingle("../big_pimpin.mp3");
                } else if (lower_low - rate) > difference {
                    lower_low = rate;
                    println!(
                        "Woah going the wrong way, setting new lower low at {}",
                        lower_low
                    );
                    play_jingle("../nooo.mp3");
                }
            }
            Err(e) => {
                eprintln!("Error fetching rate: {}", e);
                std::process::exit(-1);
            }
        }
    }
}

async fn fetch_rate(asset: &str, currency: &str) -> Result<f64, Box<dyn std::error::Error>> {
    let url = format!(
        "https://api.coinbase.com/v2/exchange-rates?currency={}",
        asset
    );

    let response = reqwest::get(&url).await?;

    if response.status().is_success() {
        let body = response.text().await?;
        let parsed_data: Response = serde_json::from_str(&body)?;

        if let Some(rate) = parsed_data.data.rates.get(currency) {
            Ok(rate.parse()?)
        } else {
            Err("Currency not found in coinbase api rates".into())
        }
    } else {
        Err(format!("Request failed with status: {}", response.status()).into())
    }
}
