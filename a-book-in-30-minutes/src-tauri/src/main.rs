fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.get(1).map(String::as_str) == Some("--e2e-materials") {
        let Some(epub_path) = args.get(2) else {
            eprintln!("Usage: a_book_in_30_minutes.exe --e2e-materials <epub-path>");
            std::process::exit(2);
        };
        match a_book_in_30_minutes_lib::run_e2e_materials_cli(epub_path) {
            Ok(()) => return,
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }
    if args.get(1).map(String::as_str) == Some("--e2e-audio") {
        let Some(epub_path) = args.get(2) else {
            eprintln!("Usage: a_book_in_30_minutes.exe --e2e-audio <epub-path>");
            std::process::exit(2);
        };
        match a_book_in_30_minutes_lib::run_e2e_audio_cli(epub_path) {
            Ok(()) => return,
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(1);
            }
        }
    }
    a_book_in_30_minutes_lib::run();
}
