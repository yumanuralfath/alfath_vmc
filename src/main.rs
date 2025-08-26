use std::{error::Error, fs::File, io::Read};

pub mod db;
pub mod model;

fn main() -> Result<(), Box<dyn Error>> {
    // Ganti dengan path file .bin milikmu
    let mut file = File::open("mc0.bin")?;

    // Baca 64 byte pertama
    let mut buffer = [0u8; 64];
    file.read_exact(&mut buffer)?;

    // Print dalam bentuk hex
    for (i, byte) in buffer.iter().enumerate() {
        print!("{byte:02X}");
        if (i + 1) % 16 == 0 {
            println!();
        }
    }

    Ok(())
}
