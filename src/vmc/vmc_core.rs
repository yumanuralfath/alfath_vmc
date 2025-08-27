use crate::model::vmc_core_model::{FSEntry, Vmc};
use std::{
    collections::HashSet,
    env,
    fs::File,
    io::{self, Read},
};

pub fn validate_mc_file(path: &str) -> io::Result<bool> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 28];
    file.read_exact(&mut buffer)?;
    Ok(&buffer == b"Sony PS2 Memory Card Format ")
}

// Struct untuk hasil ekstraksi ID game
#[derive(Debug)]
struct ExtractedId {
    id: String,
    suffix: String,
}

// Normalisasi game ID seperti di C - PERBAIKAN
fn normalize_game_id(game_id: &str) -> String {
    let mut normalized = game_id.to_uppercase();

    // Mapping prefix berdasarkan kode C yang benar
    let prefix_mappings = [
        ("BES", "SLES"), // BESLES -> SLES
        ("BAS", "SLUS"), // BASLUS -> SLUS
        ("BIS", "SLPS"), // BISLPS -> SLPS
        ("BAC", "SCUS"), // BASCUS -> SCUS
    ];

    for (old_prefix, new_prefix) in &prefix_mappings {
        if normalized.starts_with(old_prefix) {
            // Ganti hanya prefix pertama, jangan tambahkan
            normalized = normalized.replacen(old_prefix, new_prefix, 1);
            break;
        }
    }

    normalized
}

// Ekstrak game ID dari nama save seperti di C - PERBAIKAN
fn extract_game_id_from_save(save_name: &str) -> ExtractedId {
    let mut id = save_name.to_uppercase();

    // Daftar suffix yang perlu dihapus dari ID (sesuai C code)
    let suffixes = [
        "2014OPT", "2014000", "SAVEDATA", "GAMEDATA", "DAT0", "DAT1", "DAT2", "BEMU5YYY", "TCNYC",
        "000", "001", "002", "003", "004", "005", "006", "007", "008", "009", "DATA", "SAVE",
        "SYS", "SYSTEM", "CONFIG", "OPT",
    ];

    let mut found_suffix = String::new();

    // Cek suffix dari yang paling panjang ke pendek
    let mut suffixes_sorted = suffixes.to_vec();
    suffixes_sorted.sort_by_key(|s| std::cmp::Reverse(s.len()));

    for suffix in &suffixes_sorted {
        if id.ends_with(suffix) {
            let id_len = id.len() - suffix.len();
            found_suffix = suffix.to_string();
            id = id[..id_len].to_string();
            break;
        }
    }

    // JANGAN LAKUKAN NORMALISASI DI SINI - itu menyebabkan duplikasi
    // Biarkan ID asli sebagaimana adanya

    ExtractedId {
        id,
        suffix: found_suffix,
    }
}

fn get_game_title(game_id: &str, save_desc: &str) -> String {
    // Database game dengan ID asli (tanpa normalisasi)
    let games_db = [
        ("BESLES-55673", "PES 2014: Pro Evolution Soccer"),
        ("BASLUS-21050", "Burnout 3: Takedown"),
        ("BASLUS-21846", "Sonic Unleashed"),
        ("BASCUS-97436", "Gran Turismo 4"),
        ("BASLUS-21672", "Guitar Hero III: Legends of Rock"),
        ("BISLPS-25912", "Soul Eater: Battle Resonance"),
        ("BASLUS-21106", "True Crime: New York City"),
    ];

    // Ekstrak ID dan suffix tanpa normalisasi
    let extracted = extract_game_id_from_save(game_id);
    println!(
        "DEBUG: Original='{}', Extracted ID='{}', Suffix='{}'",
        game_id, extracted.id, extracted.suffix
    );

    // Cari di database - exact match atau partial match
    let base_title = games_db
        .iter()
        .find(|(id, _)| {
            // Exact match
            if *id == extracted.id {
                return true;
            }
            // Partial match - extracted ID starts with database ID
            if extracted.id.starts_with(*id) {
                return true;
            }
            // Reverse partial - database ID starts with extracted ID
            if id.starts_with(&extracted.id) {
                return true;
            }
            false
        })
        .map(|(_, title)| *title);

    match base_title {
        Some(title) => {
            // Suffix spesifik yang perlu ditampilkan
            let show_suffix = match extracted.suffix.as_str() {
                "2014OPT" | "2014000" | "DAT0" | "BEMU5YYY" | "TCNYC" => true,
                _ => false,
            };

            if show_suffix && !extracted.suffix.is_empty() {
                format!("{} ({})", title, extracted.suffix)
            } else {
                title.to_string()
            }
        }
        None => format!("Unknown Game ({})", game_id),
    }
}

pub fn print_directory_entries(entries: Vec<FSEntry>) {
    println!(
        "Save Name                        Type       Size Created          Modified         Game Title"
    );
    println!(
        "---------                        ----       ---- -------          --------         ----------"
    );

    let mut unique_games = HashSet::new();

    for entry in &entries {
        // Skip . and .. entries
        if entry.name == "." || entry.name == ".." {
            continue;
        }

        let game_id = entry.get_game_id();
        let save_desc = entry.get_save_description();
        let game_title = get_game_title(&entry.name, &save_desc); // Gunakan nama lengkap

        println!(
            "{:<32} {:<10} {:<4} {:04}/{:02}/{:02}-{:02}:{:02}:{:02} {:04}/{:02}/{:02}-{:02}:{:02}:{:02} {}",
            entry.name,
            if entry.is_directory { "DIR" } else { "FILE" },
            entry.length,
            entry.created_year,
            entry.created_month,
            entry.created_day,
            entry.created_hour,
            entry.created_min,
            entry.created_sec,
            entry.modified_year,
            entry.modified_month,
            entry.modified_day,
            entry.modified_hour,
            entry.modified_min,
            entry.modified_sec,
            game_title
        );

        // Extract base game ID untuk menghitung unique games
        let extracted = extract_game_id_from_save(&entry.name);
        unique_games.insert(extracted.id);
    }

    println!("\nTotal Games: {}", unique_games.len());
}

pub fn argument_handler() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Penggunaan: {} <file_vmc>",
            args.get(0).map_or("alfath_vmc", |s| s)
        );
        return;
    }
    let filename = &args[1];

    if !validate_mc_file(filename).unwrap_or(false) {
        eprintln!("❌ File VMC tidak valid: {filename}");
        return;
    }
    println!("✅ File VMC valid: {filename}");

    match Vmc::new(filename) {
        Ok(mut vmc) => {
            println!("\n=== Informasi VMC ===");
            println!("Versi: {}", vmc.superblock.version);
            let total_clusters = vmc.superblock.max_allocatable_clusters;
            let free_clusters = vmc.count_free_clusters();
            let used_clusters = total_clusters.saturating_sub(free_clusters);
            let cluster_size_mb = vmc.superblock.cluster_size as f64 / (1024.0 * 1024.0);
            println!(
                "Ukuran Kartu: {:.2} MB",
                total_clusters as f64 * cluster_size_mb
            );
            println!(
                "Ruang Terpakai: {:.2} MB ({} cluster)",
                used_clusters as f64 * cluster_size_mb,
                used_clusters
            );
            println!(
                "Ruang Kosong: {:.2} MB ({} cluster)",
                free_clusters as f64 * cluster_size_mb,
                free_clusters
            );
            println!("====================\n");

            println!("=== Root Directory ===");
            match vmc.list_root_directory() {
                Ok(entries) => {
                    let save_entries: Vec<_> = entries
                        .into_iter()
                        .filter(|e| e.name != "." && e.name != "..")
                        .collect();

                    if save_entries.is_empty() {
                        println!("Tidak ada save game yang ditemukan.");
                    } else {
                        print_directory_entries(save_entries);
                    }
                }
                Err(e) => eprintln!("Gagal membaca direktori: {e}"),
            }
        }
        Err(e) => eprintln!("Gagal memproses file VMC: {e}"),
    }
}
