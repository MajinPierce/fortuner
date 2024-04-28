fn main() {
    if let Err(e) = strfiler::get_args().and_then(strfiler::run) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
