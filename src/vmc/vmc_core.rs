use std::{
    env,
    fs::File,
    io::{self, Read},
};

pub fn validate_mc_file(path: &str) -> io::Result<bool> {
    let mut file = File::open(path)?;

    // buffer for read first 32 byte for header text
    let mut buffer = [0u8; 32];
    file.read_exact(&mut buffer)?;

    //convert to string lossy
    let header_text = String::from_utf8_lossy(&buffer);

    //validate_mc_file
    if header_text.contains("Sony PS2 Memory Card Format") {
        Ok(true)
    } else {
        Ok(false)
    }
}

pub fn argument_handler() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: ./alfath_vmc <filename>");
        std::process::exit(1);
    }

    let filename = &args[1];

    match validate_mc_file(filename) {
        Ok(true) => println!("✅ Validate ps2 save file: {filename}"),
        Ok(false) => println!("❌ Not validate ps2 save file: {filename}"),
        Err(e) => eprintln!("Error read file: {e}"),
    }
}
