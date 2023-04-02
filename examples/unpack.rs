use rayon::prelude::*;

fn main() -> Result<(), unpak::Error> {
    let mut args = std::env::args();
    let path = args.nth(1).unwrap_or_default();
    let key = args.next();
    let pak = unpak::Pak::new_any(path, key.as_deref().map(str::as_bytes))?;
    pak.entries()
        .into_par_iter()
        .try_for_each(|entry| -> Result<(), unpak::Error> {
            std::fs::create_dir_all(std::path::Path::new(&entry).parent().unwrap())?;
            pak.read_to_file(&entry, &entry)?;
            println!("{entry}");
            Ok(())
        })?;
    println!("done!");
    std::io::stdin().read_line(&mut String::new())?;
    Ok(())
}
