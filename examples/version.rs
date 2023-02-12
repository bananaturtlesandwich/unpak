fn main() -> Result<(), unpak::Error> {
    let mut args = std::env::args();
    let path = args.nth(1).unwrap_or_default();
    let key = args.next();
    let key = key.as_deref().map(str::as_bytes);
    println!(
        "{}",
        unpak::Pak::new_any(
            &mut std::fs::OpenOptions::new().read(true).open(&path)?,
            key
        )?
        .version()
    );
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}
