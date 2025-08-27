use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Cursor, Read, Seek, SeekFrom};
use std::path::Path;
use std::string::FromUtf8Error;

const INVALID_CLUSTER_PTR: u32 = 0xFFFFFFFF;
const EM_EXISTS: u16 = 0x8000;
const EM_DIRECTORY: u16 = 0x0010;

fn bytes_to_string(bytes: &[u8]) -> Result<String, FromUtf8Error> {
    let s = String::from_utf8(bytes.iter().copied().take_while(|&b| b != 0).collect())?;
    Ok(s)
}

// Helper functions for FAT entry interpretation (matching C code)
fn fat_flag(raw_entry: u32) -> u8 {
    ((raw_entry >> 24) & 0xFF) as u8
}

fn fat_next(raw_entry: u32) -> u32 {
    raw_entry & 0xFFFFFF
}

#[derive(Debug, Clone)]
pub struct VmcSuperblock {
    pub magic: String,
    pub version: String,
    pub page_size: i16,
    pub pages_per_cluster: u16,
    pub cluster_size: u32,
    pub clusters_per_card: u32,
    pub alloc_offset: u32,
    pub max_allocatable_clusters: u32,
    pub rootdir_cluster: u32,
    pub backup_block1: u32,
    pub backup_block2: u32,
    pub ifc_ptr_list: [u32; 32],
    pub bad_block_list: [u32; 32],
    pub cardtype: u8,
    pub cardflags: u8,
}

// VMC filesystem entry structure - harus sesuai persis dengan C struct
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct RawFSEntry {
    mode: u16,
    _pad1: u16,
    length: u32,
    // Created Time Block (Total 8 bytes)
    created_sec: u8,
    created_min: u8,
    created_hour: u8,
    created_day: u8,
    created_month: u8,
    _pad2: u8,
    created_year: u16,
    // ---
    cluster: u32,
    dir_entry: u32,
    // Modified Time Block (Total 8 bytes)
    modified_sec: u8,
    modified_min: u8,
    modified_hour: u8,
    modified_day: u8,
    modified_month: u8,
    _pad3: u8,
    modified_year: u16,
    // ---
    attr: u32,
    _pad4: [u8; 28],
    name: [u8; 32],
    _pad5: [u8; 412],
}

// Parse FS Entry dengan layout yang tepat seperti C struct
fn parse_fs_entry_from_bytes(bytes: &[u8]) -> Option<RawFSEntry> {
    if bytes.len() < 512 {
        return None;
    }

    let mut cursor = Cursor::new(bytes);

    let mode = cursor.read_u16::<LittleEndian>().ok()?;
    let _pad1 = cursor.read_u16::<LittleEndian>().ok()?;
    let length = cursor.read_u32::<LittleEndian>().ok()?;

    // Created time (8 bytes total) - berdasarkan analisis byte mentah
    let created_sec = cursor.read_u8().ok()?;
    let created_min = cursor.read_u8().ok()?;
    let created_hour = cursor.read_u8().ok()?;
    let created_day = cursor.read_u8().ok()?; // Byte ke-4 = hari
    let created_month = cursor.read_u8().ok()?; // Byte ke-5 = bulan
    let _pad2 = cursor.read_u8().ok()?;
    let created_year = cursor.read_u16::<LittleEndian>().ok()?;

    let cluster = cursor.read_u32::<LittleEndian>().ok()?;
    let dir_entry = cursor.read_u32::<LittleEndian>().ok()?;

    // Modified time (8 bytes total) - sama seperti created time
    let modified_sec = cursor.read_u8().ok()?;
    let modified_min = cursor.read_u8().ok()?;
    let modified_hour = cursor.read_u8().ok()?;
    let modified_day = cursor.read_u8().ok()?; // Byte ke-4 = hari
    let modified_month = cursor.read_u8().ok()?; // Byte ke-5 = bulan  
    let _pad3 = cursor.read_u8().ok()?;
    let modified_year = cursor.read_u16::<LittleEndian>().ok()?;

    let attr = cursor.read_u32::<LittleEndian>().ok()?;

    // Skip 28 bytes padding
    let mut _pad4 = [0u8; 28];
    cursor.read_exact(&mut _pad4).ok()?;

    // Name is at offset 64 from start, should be here
    let mut name = [0u8; 32];
    cursor.read_exact(&mut name).ok()?;

    // Remaining bytes are padding
    let remaining = 512 - 96; // 96 bytes read so far
    let mut _pad5 = vec![0u8; remaining];
    cursor.read_exact(&mut _pad5).ok()?;
    let mut pad5_array = [0u8; 412];
    pad5_array.copy_from_slice(&_pad5[..412]);

    // Fixed: Gunakan Interpretation 2 yang benar
    // sec=min, min=hour, hour=day, day=month, month=pad
    Some(RawFSEntry {
        mode,
        _pad1,
        length,
        created_sec: created_min,   // sec = byte[1] (14)
        created_min: created_hour,  // min = byte[2] (10)
        created_hour: created_day,  // hour = byte[3] (3)
        created_day: created_month, // day = byte[4] (17)
        created_month: _pad2,       // month = byte[5] (8)
        _pad2,
        created_year,
        cluster,
        dir_entry,
        modified_sec: modified_min,   // sec = byte[1]
        modified_min: modified_hour,  // min = byte[2]
        modified_hour: modified_day,  // hour = byte[3]
        modified_day: modified_month, // day = byte[4]
        modified_month: _pad3,        // month = byte[5]
        _pad3,
        modified_year,
        attr,
        _pad4,
        name,
        _pad5: pad5_array,
    })
}

// Debug tambahan untuk analisis format tanggal yang tepat
fn debug_entry_bytes(bytes: &[u8], entry_idx: usize) {
    if entry_idx == 2 {
        // Focus on Entry 2 untuk analisis
        println!("=== DETAILED DATE ANALYSIS Entry {} ===", entry_idx);
        println!("Expected from C output: 2025/08/17-03:10:14");
        println!(
            "Raw created bytes: {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X} {:02X}",
            bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
        );

        // Dekode setiap kemungkinan interpretasi
        let sec = bytes[8]; // 00
        let min = bytes[9]; // 0E = 14 
        let hour = bytes[10]; // 0A = 10
        let field3 = bytes[11]; // 03 = 3
        let field4 = bytes[12]; // 11 = 17
        let pad = bytes[13]; // 08
        let year = u16::from_le_bytes([bytes[14], bytes[15]]); // E907 = 2025

        println!(
            "Decoded: sec={}, min={}, hour={}, field3={}, field4={}, pad={}, year={}",
            sec, min, hour, field3, field4, pad, year
        );

        println!("Expected: sec=14, min=10, hour=03, day=17, month=08, year=2025");

        // Coba berbagai interpretasi
        println!(
            "Interpretation 1: sec={}, min={}, hour={}, day={}, month={}",
            sec, min, hour, field3, field4
        );
        println!(
            "Interpretation 2: sec={}, min={}, hour={}, day={}, month={}",
            min, hour, field3, field4, pad
        );
        println!(
            "Interpretation 3: sec={}, min={}, hour={}, day={}, month={}",
            hour, field3, field4, pad, sec
        );

        println!("=======================================");
    }

    println!("=== Entry {} Raw Bytes ===", entry_idx);
    println!("Mode bytes: {:02X} {:02X}", bytes[0], bytes[1]);
    println!(
        "Name start (offset 64): {:?}",
        std::str::from_utf8(&bytes[64..96]).unwrap_or("invalid")
    );
    println!("========================");
}

// Debug helper untuk melihat data mentah
fn debug_raw_entry(raw: &RawFSEntry, index: usize) {
    println!(
        "Entry {}: mode=0x{:04X}, length={}, name={:?}",
        index,
        u16::from_le(raw.mode),
        u32::from_le(raw.length),
        std::str::from_utf8(&raw.name).unwrap_or("invalid utf8")
    );
}

impl VmcSuperblock {
    pub fn from_reader<R: Read>(reader: &mut R) -> io::Result<Self> {
        let mut buf = [0u8; 384];
        reader.read_exact(&mut buf)?;
        let mut cursor = Cursor::new(&buf[..]);

        let mut magic_buf = [0u8; 28];
        cursor.read_exact(&mut magic_buf)?;
        if &magic_buf[..] != b"Sony PS2 Memory Card Format " {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Magic string tidak valid",
            ));
        }

        let mut version_buf = [0u8; 12];
        cursor.read_exact(&mut version_buf)?;

        cursor.seek(SeekFrom::Start(0x28))?;
        let page_size = cursor.read_i16::<LittleEndian>()?;
        let pages_per_cluster = cursor.read_u16::<LittleEndian>()?;

        cursor.seek(SeekFrom::Start(0x34))?;
        let alloc_offset = cursor.read_u32::<LittleEndian>()?;

        cursor.seek(SeekFrom::Start(0x3C))?;
        let rootdir_cluster = cursor.read_u32::<LittleEndian>()?;
        let backup_block1 = cursor.read_u32::<LittleEndian>()?;
        let backup_block2 = cursor.read_u32::<LittleEndian>()?;

        cursor.seek(SeekFrom::Start(0x50))?;
        let mut ifc_ptr_list = [0u32; 32];
        cursor.read_u32_into::<LittleEndian>(&mut ifc_ptr_list)?;

        cursor.seek(SeekFrom::Start(0xD0))?;
        let mut bad_block_list = [0u32; 32];
        cursor.read_u32_into::<LittleEndian>(&mut bad_block_list)?;

        cursor.seek(SeekFrom::Start(0x150))?;
        let cardtype = cursor.read_u8()?;
        let cardflags = cursor.read_u8()?;

        cursor.seek(SeekFrom::Start(0x154))?;
        let cluster_size = cursor.read_u32::<LittleEndian>()?;

        cursor.seek(SeekFrom::Start(0x170))?;
        let max_allocatable_clusters = cursor.read_u32::<LittleEndian>()?;

        Ok(VmcSuperblock {
            magic: bytes_to_string(&magic_buf).unwrap_or_default(),
            version: bytes_to_string(&version_buf).unwrap_or_default(),
            page_size,
            pages_per_cluster,
            cluster_size,
            clusters_per_card: 65536,
            alloc_offset,
            max_allocatable_clusters,
            rootdir_cluster,
            backup_block1,
            backup_block2,
            ifc_ptr_list,
            bad_block_list,
            cardtype,
            cardflags,
        })
    }
}

#[derive(Debug, Clone)]
pub struct FSEntry {
    pub name: String,
    pub mode: u16,
    pub length: u32,
    pub cluster: u32,
    pub is_directory: bool,
    pub created_sec: u8,
    pub created_min: u8,
    pub created_hour: u8,
    pub created_day: u8,
    pub created_month: u8,
    pub created_year: u16,
    pub modified_sec: u8,
    pub modified_min: u8,
    pub modified_hour: u8,
    pub modified_day: u8,
    pub modified_month: u8,
    pub modified_year: u16,
}

impl FSEntry {
    fn from_raw(raw: &RawFSEntry) -> Option<Self> {
        // Convert from little endian
        let mode_val = raw.mode; // Sudah little endian dari parsing
        let exists = (mode_val & EM_EXISTS) != 0;

        if !exists {
            return None;
        }

        let name_bytes = raw.name;
        let name = bytes_to_string(&name_bytes).unwrap_or_default();
        if name.is_empty() {
            return None;
        }

        // Analisis mode 0x8427:
        // 0x8427 = 0x8000 (EM_EXISTS) + 0x0400 + 0x0020 + 0x0007
        // Ternyata 0x8427 mengandung directory flag di bit yang berbeda
        // Berdasarkan output C, semua entry adalah DIR, jadi 0x8427 berarti directory
        let is_directory = mode_val == 0x8427 || (mode_val & EM_DIRECTORY) != 0;

        Some(FSEntry {
            name,
            mode: mode_val,
            length: raw.length,   // Sudah little endian dari parsing
            cluster: raw.cluster, // Sudah little endian dari parsing
            is_directory,
            created_sec: raw.created_sec,
            created_min: raw.created_min,
            created_hour: raw.created_hour,
            created_day: raw.created_day,
            created_month: raw.created_month,
            created_year: raw.created_year, // Sudah little endian dari parsing
            modified_sec: raw.modified_sec,
            modified_min: raw.modified_min,
            modified_hour: raw.modified_hour,
            modified_day: raw.modified_day,
            modified_month: raw.modified_month,
            modified_year: raw.modified_year, // Sudah little endian dari parsing
        })
    }

    pub fn get_game_id(&self) -> String {
        // Extract game ID (before the first non-alphanumeric character after the initial part)
        let name = &self.name;
        if let Some(pos) = name.find(|c: char| !c.is_alphanumeric() && c != '-') {
            name[..pos].to_string()
        } else {
            name.clone()
        }
    }

    pub fn get_save_description(&self) -> String {
        // Extract description part after game ID
        let name = &self.name;
        if let Some(pos) = name.find(|c: char| !c.is_alphanumeric() && c != '-') {
            name[pos..].to_string()
        } else {
            String::new()
        }
    }
}

pub struct FatTable {
    pub fat: Vec<u32>,
}

pub struct Vmc {
    file: File,
    pub superblock: VmcSuperblock,
    fat: FatTable,
}

impl Vmc {
    pub fn new<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let mut file = File::open(path)?;
        let superblock = VmcSuperblock::from_reader(&mut file)?;
        let fat = Self::load_fat(&mut file, &superblock)?;
        Ok(Vmc {
            file,
            superblock,
            fat,
        })
    }

    fn load_fat(file: &mut File, sb: &VmcSuperblock) -> io::Result<FatTable> {
        let entries_per_cluster = sb.cluster_size as usize / 4;
        let mut fat_cluster_ptrs = Vec::new();

        // Load FAT cluster pointers from IFC list
        for &ifc in &sb.ifc_ptr_list {
            if ifc == 0 || ifc == INVALID_CLUSTER_PTR {
                break;
            }
            file.seek(SeekFrom::Start(ifc as u64 * sb.cluster_size as u64))?;
            for _ in 0..entries_per_cluster {
                let entry = file.read_u32::<LittleEndian>()?;
                if entry == INVALID_CLUSTER_PTR {
                    break;
                }
                fat_cluster_ptrs.push(entry);
            }
        }

        // Load actual FAT data from the clusters pointed to by fat_cluster_ptrs
        let mut fat = Vec::with_capacity(fat_cluster_ptrs.len() * entries_per_cluster);
        for &fat_ptr in &fat_cluster_ptrs {
            file.seek(SeekFrom::Start(fat_ptr as u64 * sb.cluster_size as u64))?;
            for _ in 0..entries_per_cluster {
                fat.push(file.read_u32::<LittleEndian>()?);
            }
        }
        Ok(FatTable { fat })
    }

    // Fixed free cluster counting using the same logic as C code
    pub fn count_free_clusters(&self) -> u32 {
        let mut free_count = 0;
        for &raw_entry in &self.fat.fat {
            let flag = fat_flag(raw_entry);
            let cluster = fat_next(raw_entry);

            // Check if cluster is free (flag == 0x7F and cluster == 0xFFFFFF indicates free)
            if flag == 0x7F && cluster == 0xFFFFFF {
                free_count += 1;
            }
        }
        free_count
    }

    // Build cluster chain like C code
    fn build_cluster_chain(&self, start_cluster: u32) -> Vec<u32> {
        let mut chain = Vec::new();
        let mut current = start_cluster;
        let mut processed = HashSet::new();

        while current != INVALID_CLUSTER_PTR && !processed.contains(&current) {
            processed.insert(current);
            chain.push(current);

            if (current as usize) >= self.fat.fat.len() {
                break;
            }

            let raw_entry = self.fat.fat[current as usize];
            let flag = fat_flag(raw_entry);

            if flag == 0xFF {
                break; // End of chain
            }

            current = fat_next(raw_entry);
        }

        chain
    }

    pub fn list_root_directory(&mut self) -> io::Result<Vec<FSEntry>> {
        let file_size = self.file.metadata()?.len();
        let root_offset = (self.superblock.alloc_offset + self.superblock.rootdir_cluster) as u64
            * self.superblock.cluster_size as u64;

        if root_offset >= file_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Root directory offset exceeds file size",
            ));
        }

        // Read the root directory header to get expected entry count
        self.file.seek(SeekFrom::Start(root_offset))?;
        let entry_size = std::mem::size_of::<RawFSEntry>();
        let mut header_buf = vec![0u8; entry_size];
        self.file.read_exact(&mut header_buf)?;
        let root_hdr: RawFSEntry = unsafe { std::ptr::read(header_buf.as_ptr() as *const _) };
        let expected_len = u32::from_le(root_hdr.length);

        println!("Expected entries: {}", expected_len);
        println!("Entry size: {} bytes", entry_size);

        // Build cluster chain for root directory (like C code)
        let cluster_chain = self.build_cluster_chain(self.superblock.rootdir_cluster);
        println!("Cluster chain: {:?}", cluster_chain);

        // VMC entries are actually 512 bytes each, but we need to read them correctly
        let vmc_entry_size = 512; // Each VMC entry is 512 bytes
        let entries_per_cluster = self.superblock.cluster_size as usize / vmc_entry_size;
        println!("Entries per cluster: {}", entries_per_cluster);

        let mut entries = Vec::new();
        let mut read_count = 0;

        // Process each cluster in the chain
        for (cluster_idx, &cluster) in cluster_chain.iter().enumerate() {
            let cluster_offset = (self.superblock.alloc_offset + cluster) as u64
                * self.superblock.cluster_size as u64;

            if cluster_offset >= file_size {
                println!("Cluster {} offset exceeds file size", cluster);
                break;
            }

            println!(
                "Processing cluster {} at offset 0x{:X}",
                cluster, cluster_offset
            );
            self.file.seek(SeekFrom::Start(cluster_offset))?;

            // Read entire cluster into buffer
            let mut cluster_buf = vec![0u8; self.superblock.cluster_size as usize];
            self.file.read_exact(&mut cluster_buf)?;

            for i in 0..entries_per_cluster {
                if read_count >= expected_len {
                    println!("Reached expected entry count");
                    break;
                }

                let entry_start = i * vmc_entry_size;
                if entry_start + vmc_entry_size > cluster_buf.len() {
                    println!("Entry {} would exceed cluster buffer", i);
                    break;
                }

                // Parse entry menggunakan manual parsing dengan ukuran 512 bytes
                let entry_bytes = &cluster_buf[entry_start..entry_start + vmc_entry_size];

                // Debug raw bytes terlebih dahulu
                debug_entry_bytes(entry_bytes, read_count as usize);

                if let Some(raw_entry) = parse_fs_entry_from_bytes(entry_bytes) {
                    // Debug setiap entry
                    debug_raw_entry(&raw_entry, read_count as usize);
                    read_count += 1;

                    if let Some(entry) = FSEntry::from_raw(&raw_entry) {
                        println!(
                            "Valid entry found: {} ({})",
                            entry.name,
                            if entry.is_directory { "DIR" } else { "FILE" }
                        );
                        entries.push(entry);
                    } else {
                        println!("Entry {} not valid or doesn't exist", read_count - 1);
                    }
                } else {
                    println!("Failed to parse entry {}", read_count);
                    read_count += 1;
                }
            }

            if read_count >= expected_len {
                break;
            }
        }

        println!(
            "Total entries processed: {}, valid entries: {}",
            read_count,
            entries.len()
        );
        Ok(entries)
    }
}
