fn main() -> std::process::ExitCode {
    let map_name = std::env::args()
        .skip(1)
        .find_map(|argument| {
            argument
                .strip_prefix("map=")
                .or_else(|| argument.strip_prefix("--map="))
                .map(str::to_owned)
        })
        .unwrap_or_else(|| "intro".to_owned());
    let Some(map) = terminal::maps::build_named_map(&map_name) else {
        eprintln!("unknown map name '{map_name}'. Available maps: intro, map01");
        return std::process::ExitCode::FAILURE;
    };
    let configuration = terminal::app::fetch_configuration();
    terminal::app::build_gui_app(configuration, map).run();
    std::process::ExitCode::SUCCESS
}
