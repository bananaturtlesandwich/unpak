use rayon::iter::ParallelIterator;

fn main() -> Result<(), unpak::Error> {
    let mut args = std::env::args();
    let path = args.nth(1).unwrap_or_default();
    let key = args.next();
    unpak::Pak::new_from_path(path, key.as_deref().map(str::as_bytes))?
        .par_entries()
        .for_each(|entry| println!("{entry}"));
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}
