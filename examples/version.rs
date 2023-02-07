fn main() -> Result<(), unpak::Error> {
    let mut args = std::env::args();
    let path = args.nth(1).unwrap_or_default();
    let key = args.next();
    let key = key.as_deref().map(str::as_bytes);
    println!(
        "{}",
        unpak::Pak::load(
            || std::fs::OpenOptions::new().read(true).open(&path).ok(),
            key
        )?
        .version()
    );
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}
