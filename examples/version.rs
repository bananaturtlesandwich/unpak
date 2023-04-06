fn main() {
    let mut args = std::env::args();
    let path = args.nth(1).unwrap_or_default();
    let mut key = None;
    if let Some(hash) = args.next() {
        match hex::decode(hash) {
            Ok(bytes) => key = Some(bytes),
            Err(e) => {
                eprintln!("hex: {e}");
                std::io::stdin().read_line(&mut String::new()).unwrap();
                return;
            }
        }
    }
    match unpak::Pak::new_any(path, key.as_deref()) {
        Ok(pak) => println!("{}", pak.version()),
        Err(e) => eprintln!("{e}"),
    }
    std::io::stdin().read_line(&mut String::new()).unwrap();
}
