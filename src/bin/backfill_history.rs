use std::path::PathBuf;

use chrono::{Duration, Local, NaiveDate};
use dotenvy::dotenv;

use agile_fetcher::agile::fetch_and_store_agile_for_day;
use agile_fetcher::history::{fetch_and_store_history_for_day, load_octopus_config};
use agile_fetcher::home_assistant::load_ha_config;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let args: Vec<String> = std::env::args().collect();
    let days_back: i64 = args
        .get(1)
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(180);

    let agile_dir = PathBuf::from("data/agile");
    let history_dir = PathBuf::from("data/history");

    let ha_config = load_ha_config().expect("Failed to load Home Assistant config");
    let octopus_config = load_octopus_config().expect("Failed to load Octopus config");

    let today = Local::now().date_naive();
    let start_day = today - Duration::days(days_back);

    println!(
        "Backfilling history from {} to {}",
        start_day,
        today - Duration::days(1)
    );

    let mut current: NaiveDate = start_day;

    while current < today {
        println!("----------------------------------------");
        println!("Backfilling {}", current);

        match fetch_and_store_agile_for_day(&agile_dir, current).await {
            Ok(()) => {
                println!("Agile saved for {}", current);
            }
            Err(err) => {
                eprintln!("Failed to fetch Agile for {}: {}", current, err);
                current += Duration::days(1);
                continue;
            }
        }

        match fetch_and_store_history_for_day(
            &history_dir,
            &agile_dir,
            &octopus_config,
            current,
            ha_config.dev_mode,
        )
        .await
        {
            Ok(()) => {
                println!("History saved for {}", current);
            }
            Err(err) => {
                eprintln!("Failed to fetch history for {}: {}", current, err);
            }
        }

        current += Duration::days(1);

        // be polite to APIs
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    println!("----------------------------------------");
    println!("Backfill complete");
}
