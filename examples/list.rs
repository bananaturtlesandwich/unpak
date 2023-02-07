fn main() -> Result<(), unpak::Error> {
    let mut args = std::env::args();
    let path = args.nth(1).unwrap_or_default();
    let key = args.next();
    for file in unpak::Pak::load(
        || std::fs::OpenOptions::new().read(true).open(&path).ok(),
        key,
    )?
    .files()
    {
        println!("{file}");
    }
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}
