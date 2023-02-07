fn main() -> Result<(), unpak::Error> {
    let mut args = std::env::args();
    let path = args.nth(1).unwrap_or_default();
    let key = args.next();
    let key = key.as_deref().map(str::as_bytes);
    let mut pak = unpak::Pak::load(
        || std::fs::OpenOptions::new().read(true).open(&path).ok(),
        key,
    )?;
    for file in pak.files() {
        std::fs::create_dir_all(
            std::path::Path::new(file.trim_start_matches('/'))
                .parent()
                .expect("will be a file"),
        )?;
        match pak.read(
            &file,
            &mut std::fs::File::create(file.trim_start_matches('/'))?,
        ) {
            Ok(_) => println!("{file}"),
            Err(e) => eprintln!("{e}"),
        }
    }
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}
