fn main() {
    let result = toon::cli::run();
    if let Err(err) = result {
        // `eprintln!` panics when stderr cannot be written, which the abort-on-panic release
        // profile turned into SIGABRT; the exit status must still be 1.
        toon::cli::report(&err.to_string());
        std::process::exit(1);
    }
}
