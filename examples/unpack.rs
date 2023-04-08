use rayon::prelude::*;

fn main() {
    let mut args = std::env::args();
    let path = args.nth(1).unwrap_or_default();
    let mut key = None;
    if let Some(hash) = args.next() {
        match hex::decode(hash.trim_start_matches("0x")) {
            Ok(bytes) => key = Some(bytes),
            Err(e) => {
                eprintln!("hex: {e}");
                std::io::stdin().read_line(&mut String::new()).unwrap();
                return;
            }
        }
    }
    match unpack(&path, key.as_deref()) {
        Ok(_) => println!("unpacked successfully"),
        Err(e) => eprintln!("{e}"),
    }
    std::io::stdin().read_line(&mut String::new()).unwrap();
}

fn unpack(path: &str, key: Option<&[u8]>) -> Result<(), unpak::Error> {
    let pak = unpak::Pak::new_any(path, key)?;
    pak.entries()
        .into_par_iter()
        .try_for_each(|entry| -> Result<(), unpak::Error> {
            std::fs::create_dir_all(std::path::Path::new(&entry).parent().unwrap())?;
            pak.read_to_file(&entry, &entry)?;
            println!("{entry}");
            Ok(())
        })?;
    Ok(())
}
