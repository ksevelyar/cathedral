fn main() {
    let configuration = cathedral::app::fetch_configuration();
    cathedral::app::build_gui_app(configuration).run();
}
