use rayon::iter::ParallelIterator;
use rayon::prelude::*;
use std::fs;

fn main() -> Result<(), unpak::Error> {
    let mut args = std::env::args();
    let path = args.nth(1).unwrap_or_default();
    let key = args.next();
    let pak = unpak::Pak::new_any(path, key.as_deref().map(str::as_bytes))?;
    pak.entries()
        .into_par_iter()
        .try_for_each(|entry| -> Result<(), unpak::Error> {
            let trimmed_path = entry.trim_start_matches('/');
            // the parent will always be a file
            fs::create_dir_all(std::path::Path::new(trimmed_path).parent().unwrap())?;
            pak.read_to_file(&entry, trimmed_path)?;
            println!("{entry}");
            Ok(())
        })?;
    println!("done!");
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}
