pub fn unpack(path: String, key: Option<String>) -> Result<(), unpak::Error> {
    let mut pak = super::load_pak(path.clone(), key)?;
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
    Ok(())
}
