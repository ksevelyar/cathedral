use std::env;
use std::process;

fn main() {
    let show_fps = parse_show_fps_argument();
    cathedral::app::build_gui_app(show_fps).run();
}

fn parse_show_fps_argument() -> bool {
    let arguments: Vec<String> = env::args().skip(1).collect();
    match arguments.as_slice() {
        [] => false,
        [argument] if argument == "--fps" => true,
        _ => {
            eprintln!("usage: cathedral [--fps]");
            process::exit(1);
        }
    }
}
