fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let Err(e) = faxina_lib::cli::run(&args, &mut std::io::stdout().lock()) {
        eprintln!("{e}");
        std::process::exit(2);
    }
}
