fn main() {
    let result = tru::cli::run();
    if let Err(err) = result {
        eprintln!("{err}");
        std::process::exit(1);
    }
}
