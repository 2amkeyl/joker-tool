fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    if std::env::var("CARGO_CFG_TARGET_OS").map(|v| v == "windows").unwrap_or(false) {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Joker Tool");
        res.set("FileDescription", "Joker Tool");
        res.compile().unwrap();
    }
}
