fn main() {
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("assets/peel.ico");
        res.compile().expect("failed to compile windows resources");
    }
}
