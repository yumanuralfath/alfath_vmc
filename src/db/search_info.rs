use crate::model::db_struct::TitleEntry;
use csv::ReaderBuilder;
use std::env;
use std::io::{self, Write};
use std::path::PathBuf;
use std::{error::Error, fs::File};

pub fn load_data_from_tsv(query: &str, path: &str) -> Result<Vec<TitleEntry>, Box<dyn Error>> {
    let file = File::open(path)?;
    let mut rdr = ReaderBuilder::new().delimiter(b'\t').from_reader(file);

    let mut results = Vec::new();

    for result in rdr.deserialize::<TitleEntry>() {
        let record = result?;
        if record.id.to_lowercase().contains(&query.to_lowercase())
            || record.title.to_lowercase().contains(&query.to_lowercase())
        {
            results.push(record);
        }
    }
    Ok(results)
}

pub fn search_info_from_id(id: &str) -> Result<TitleEntry, String> {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("PS2.data.tsv");

    match load_data_from_tsv(id, path.to_str().unwrap()) {
        Ok(mut results) => {
            if let Some(entry) = results.pop() {
                Ok(entry)
            } else {
                Err(format!("ID {id} not found"))
            }
        }
        Err(e) => Err(e.to_string()),
    }
}

pub fn input_handler() -> Result<TitleEntry, String> {
    print!("Please insert your game id: ");
    io::stdout().flush().expect("Failed to flush stdout");

    let mut input_string = String::new();
    io::stdin()
        .read_line(&mut input_string)
        .expect("Failed to read input");

    let trimmed_input = input_string.trim();
    search_info_from_id(trimmed_input)
}

pub fn info_game_ps2() {
    println!("=== PS2 GAME INFO ===");

    match input_handler() {
        Ok(game_info) => {
            println!("\n ✅ Game Found!");
            println!("ID: {}", game_info.id);
            println!("Title: {}", game_info.title);
            println!("Developer: {}", game_info.developer);
            println!("Genre: {}", game_info.genre);
            println!("Language: {}", game_info.language);
            println!("publisher: {}", game_info.publisher);
            println!("Region: {}", game_info.region);
            println!("Release Date: {}", game_info.release_date);
        }
        Err(error) => {
            println!("\n❌ Error: {error}");
        }
    }
}
