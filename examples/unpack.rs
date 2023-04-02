use rayon::iter::ParallelIterator;
use rayon::prelude::*;
use std::fs;

fn main() -> Result<(), unpak::Error> {
    let mut args = std::env::args();
    let path = args.nth(1).unwrap_or_default();
    let game = path.trim_end_matches(".pak").to_string() + "/Content";
    let key = args.next();
    let pak = unpak::Pak::new_any(path, key.as_deref().map(str::as_bytes))?;
    pak.entries()
        .into_par_iter()
        .try_for_each(|entry| -> Result<(), unpak::Error> {
            // undo the stuff we did with names for asset interoperability
            let path = entry.trim_start_matches('/').replacen("Game", &game, 1);
            fs::create_dir_all(std::path::Path::new(&path).parent().unwrap())?;
            pak.read_to_file(&entry, &path)?;
            println!("{entry}");
            Ok(())
        })?;
    println!("done!");
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}
