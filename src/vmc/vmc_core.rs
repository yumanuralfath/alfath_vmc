use crate::model::vmc_core_model::{FSEntry, Vmc};
use crate::vmc::search_info::search_info_from_id;
use std::io::Seek;
use std::{
    collections::HashSet,
    env,
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};

pub fn validate_mc_file(path: &str) -> io::Result<bool> {
    let mut file = File::open(path)?;
    let mut buffer = [0u8; 28];
    file.read_exact(&mut buffer)?;
    Ok(&buffer == b"Sony PS2 Memory Card Format ")
}

#[derive(Debug)]
pub struct ExtractedId {
    pub id: String,
    pub suffix: String,
}

pub fn extract_game_id_from_save(save_name: &str) -> ExtractedId {
    let mut id = save_name.to_uppercase();

    let suffixes = [
        "2014OPT", "2014000", "SAVEDATA", "GAMEDATA", "DAT0", "DAT1", "DAT2", "BEMU5YYY", "TCNYC",
        "000", "001", "002", "003", "004", "005", "006", "007", "008", "009", "DATA", "SAVE",
        "SYS", "SYSTEM", "CONFIG", "OPT",
    ];

    let mut found_suffix = String::new();
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

    ExtractedId {
        id,
        suffix: found_suffix,
    }
}

// Updated function to use dynamic lookup from TSV
pub fn get_game_title(save_name: &str) -> String {
    let extracted = extract_game_id_from_save(save_name);

    // Try to get game info from TSV database
    match search_info_from_id(&extracted.id) {
        Ok(game_info) => {
            let show_suffix = matches!(
                extracted.suffix.as_str(),
                "2014OPT" | "2014000" | "DAT0" | "BEMU5YYY" | "TCNYC"
            );

            if show_suffix && !extracted.suffix.is_empty() {
                format!("{} ({})", game_info.title, extracted.suffix)
            } else {
                game_info.title
            }
        }
        Err(_) => {
            // Fallback to hardcoded database if TSV lookup fails
            let games_db = [
                ("BESLES-55673", "PES 2014: Pro Evolution Soccer"),
                ("BASLUS-21050", "Burnout 3: Takedown"),
                ("BASLUS-21846", "Sonic Unleashed"),
                ("BASCUS-97436", "Gran Turismo 4"),
                ("BASLUS-21672", "Guitar Hero III: Legends of Rock"),
                ("BISLPS-25912", "Soul Eater: Battle Resonance"),
                ("BASLUS-21106", "True Crime: New York City"),
            ];

            let base_title = games_db
                .iter()
                .find(|(id, _)| {
                    *id == extracted.id
                        || extracted.id.starts_with(*id)
                        || id.starts_with(&extracted.id)
                })
                .map(|(_, title)| *title);

            match base_title {
                Some(title) => {
                    let show_suffix = matches!(
                        extracted.suffix.as_str(),
                        "2014OPT" | "2014000" | "DAT0" | "BEMU5YYY" | "TCNYC"
                    );

                    if show_suffix && !extracted.suffix.is_empty() {
                        format!("{} ({})", title, extracted.suffix)
                    } else {
                        title.to_string()
                    }
                }
                None => format!("Unknown Game ({save_name})"),
            }
        }
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
        if entry.name == "." || entry.name == ".." {
            continue;
        }

        let game_title = get_game_title(&entry.name);

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

        let extracted = extract_game_id_from_save(&entry.name);
        unique_games.insert(extracted.id);
    }

    println!("\nTotal Games: {}", unique_games.len());
}

// New function to extract save directories from VMC
pub fn extract_save_directories(vmc: &mut Vmc, output_dir: &str) -> io::Result<()> {
    println!("🔄 Extracting save directories...");

    // Create output directory if it doesn't exist
    fs::create_dir_all(output_dir)?;

    let entries = vmc.list_root_directory()?;
    let mut extracted_count = 0;

    for entry in entries {
        if entry.name == "." || entry.name == ".." {
            continue;
        }

        if entry.is_directory {
            let save_dir = Path::new(output_dir).join(&entry.name);

            println!("📁 Extracting directory: {}", entry.name);

            match extract_directory_contents(vmc, entry.cluster, &save_dir) {
                Ok(files_count) => {
                    println!("   ✅ Extracted {files_count} files");
                    extracted_count += 1;
                }
                Err(e) => {
                    eprintln!("   ❌ Failed to extract {}: {}", entry.name, e);
                }
            }
        }
    }

    println!("\n🎉 Successfully extracted {extracted_count} save directories to '{output_dir}'",);
    Ok(())
}

// Helper function to extract directory contents
fn extract_directory_contents(
    vmc: &mut Vmc,
    start_cluster: u32,
    output_dir: &Path,
) -> io::Result<usize> {
    fs::create_dir_all(output_dir)?;

    // Read directory entries from the cluster chain
    let cluster_chain = vmc.build_cluster_chain(start_cluster);
    let vmc_entry_size = 512;
    let entries_per_cluster = vmc.superblock.cluster_size as usize / vmc_entry_size;

    let mut file_count = 0;
    let mut entry_count = 0;

    println!("   🔗 Cluster chain: {cluster_chain:?}");

    // Let's examine the first cluster more carefully
    if let Some(&first_cluster) = cluster_chain.first() {
        let first_cluster_offset = (vmc.superblock.alloc_offset + first_cluster) as u64
            * vmc.superblock.cluster_size as u64;

        vmc.file
            .seek(std::io::SeekFrom::Start(first_cluster_offset))?;
        let mut header_buf = vec![0u8; vmc_entry_size];
        vmc.file.read_exact(&mut header_buf)?;

        // Debug: print first few bytes of header
        println!("   🔍 Header bytes: {:02X?}", &header_buf[..32]);

        // Try to parse header differently - maybe it's not a standard FS entry
        let header_length = if let Some(header) =
            crate::model::vmc_core_model::parse_fs_entry_from_bytes(&header_buf)
        {
            println!("   📊 Header parsed - length field: {}", header.length);
            header.length as usize
        } else {
            println!("   ⚠️  Header parsing failed, using fallback");
            // Fallback: scan all entries in all clusters
            entries_per_cluster * cluster_chain.len()
        };

        println!("   📊 Expected entries in directory: {header_length}");
    }

    // Process all clusters, starting from the first one
    for &cluster in &cluster_chain {
        let cluster_offset =
            (vmc.superblock.alloc_offset + cluster) as u64 * vmc.superblock.cluster_size as u64;

        vmc.file.seek(std::io::SeekFrom::Start(cluster_offset))?;

        let mut cluster_buf = vec![0u8; vmc.superblock.cluster_size as usize];
        vmc.file.read_exact(&mut cluster_buf)?;

        for i in 0..entries_per_cluster {
            let entry_start = i * vmc_entry_size;
            if entry_start + vmc_entry_size > cluster_buf.len() {
                break;
            }

            let entry_bytes = &cluster_buf[entry_start..entry_start + vmc_entry_size];

            // Check if entry is all zeros (empty entry)
            if entry_bytes.iter().all(|&b| b == 0) {
                continue;
            }

            // Debug: print first few bytes of each entry
            if entry_count < 10 {
                // Only print first 10 for brevity
                println!(
                    "   🔍 Entry {} bytes: {:02X?}",
                    entry_count,
                    &entry_bytes[..32]
                );
            }

            if let Some(raw_entry) =
                crate::model::vmc_core_model::parse_fs_entry_from_bytes(entry_bytes)
            {
                entry_count += 1;

                if let Some(fs_entry) = FSEntry::from_raw(&raw_entry) {
                    println!(
                        "   🔍 Found entry: '{}' (dir: {}, cluster: {}, size: {}, mode: 0x{:04X})",
                        fs_entry.name,
                        fs_entry.is_directory,
                        fs_entry.cluster,
                        fs_entry.length,
                        fs_entry.mode
                    );

                    if fs_entry.name != "." && fs_entry.name != ".." {
                        if fs_entry.is_directory {
                            println!("   📁 Skipping directory: {}", fs_entry.name);
                        } else if fs_entry.cluster > 0 && fs_entry.cluster != 0xFFFFFFFF {
                            // Extract file data
                            println!("   💾 Extracting file: {}", fs_entry.name);
                            match extract_file_data(vmc, fs_entry.cluster, fs_entry.length) {
                                Ok(file_data) => {
                                    if !file_data.is_empty() {
                                        let file_path = output_dir.join(&fs_entry.name);
                                        let mut output_file = File::create(&file_path)?;
                                        output_file.write_all(&file_data)?;
                                        println!(
                                            "   ✅ Successfully extracted: {} ({} bytes)",
                                            fs_entry.name,
                                            file_data.len()
                                        );
                                        file_count += 1;
                                    } else {
                                        println!("   ⚠️  File {} is empty", fs_entry.name);
                                    }
                                }
                                Err(e) => {
                                    eprintln!(
                                        "   ❌ Failed to extract file {}: {}",
                                        fs_entry.name, e
                                    );
                                }
                            }
                        } else {
                            println!(
                                "   ⚠️  File {} has invalid cluster: {}",
                                fs_entry.name, fs_entry.cluster
                            );
                        }
                    }
                } else {
                    // Raw entry parsed but FSEntry::from_raw returned None
                    println!(
                        "   ⚠️  Raw entry parsed but FSEntry::from_raw failed (mode: 0x{:04X})",
                        raw_entry.mode
                    );
                }
            } else {
                // Check if there are any non-zero bytes that might indicate data
                let non_zero_count = entry_bytes.iter().filter(|&&b| b != 0).count();
                if non_zero_count > 0 && entry_count < 10 {
                    println!(
                        "   ⚠️  Entry {entry_count} failed to parse but has {non_zero_count} non-zero bytes",
                    );
                }
            }
        }
    }

    println!("   📈 Processed {entry_count} entries total");
    Ok(file_count)
}

// Helper function to extract file data
fn extract_file_data(vmc: &mut Vmc, start_cluster: u32, file_size: u32) -> io::Result<Vec<u8>> {
    if start_cluster == 0 || start_cluster == 0xFFFFFFFF {
        return Ok(Vec::new());
    }

    let cluster_chain = vmc.build_cluster_chain(start_cluster);
    let mut file_data = Vec::with_capacity(file_size as usize);
    let mut bytes_read = 0u32;

    println!("     🔗 Cluster chain: {cluster_chain:?}");
    println!("     📏 File size: {file_size} bytes");

    for &cluster in cluster_chain.iter() {
        if bytes_read >= file_size {
            break;
        }

        let cluster_offset =
            (vmc.superblock.alloc_offset + cluster) as u64 * vmc.superblock.cluster_size as u64;

        println!("     📍 Reading cluster {cluster} at offset 0x{cluster_offset:X}",);

        vmc.file.seek(std::io::SeekFrom::Start(cluster_offset))?;

        let bytes_to_read = std::cmp::min(vmc.superblock.cluster_size, file_size - bytes_read);

        let mut cluster_data = vec![0u8; bytes_to_read as usize];
        vmc.file.read_exact(&mut cluster_data)?;

        file_data.extend_from_slice(&cluster_data);
        bytes_read += bytes_to_read;

        println!("     ✅ Read {bytes_to_read} bytes from cluster {cluster}",);
    }

    println!("     📊 Total bytes read: {}", file_data.len());
    Ok(file_data)
}

pub fn argument_handler() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Penggunaan: {} <file_vmc> [extract <output_dir>]",
            args.first().map_or("alfath_vmc", |s| s)
        );
        eprintln!("  <file_vmc>     : Path to VMC file");
        eprintln!("  extract        : Extract save directories");
        eprintln!("  <output_dir>   : Output directory for extraction (default: extracted_saves)");
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

            // Check if extraction is requested
            if args.len() >= 3 && args[2] == "extract" {
                let output_dir = if args.len() >= 4 {
                    &args[3]
                } else {
                    "extracted_saves"
                };

                if let Err(e) = extract_save_directories(&mut vmc, output_dir) {
                    eprintln!("❌ Gagal mengekstrak save directories: {e}");
                }
            } else {
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

                        println!(
                            "\n💡 Tip: Gunakan 'extract <output_dir>' untuk mengekstrak save directories"
                        );
                    }
                    Err(e) => eprintln!("Gagal membaca direktori: {e}"),
                }
            }
        }
        Err(e) => eprintln!("Gagal memproses file VMC: {e}"),
    }
}
