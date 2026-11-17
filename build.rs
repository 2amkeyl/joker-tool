use std::env;
use std::fs;
use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");

    let houses = [
        ("BRILLIANCE", "assets/hypesquadbrilliance.png"),
        ("BRAVERY", "assets/hypesquadbravery.png"),
        ("BALANCE", "assets/hypesquadbalance.png"),
    ];
    let out_dir = env::var_os("OUT_DIR").expect("OUT_DIR is set by Cargo");
    let out_dir = Path::new(&out_dir);
    let mut generated = String::new();
    for (name, relative) in houses {
        println!("cargo:rerun-if-changed={relative}");
        let filename = Path::new(relative).file_name().expect("house icon filename");
        let value = if Path::new(relative).is_file() {
            fs::copy(relative, out_dir.join(filename)).expect("copy HypeSquad PNG into OUT_DIR");
            format!("Some(include_bytes!(concat!(env!(\"OUT_DIR\"), \"/{}\")))", filename.to_string_lossy())
        } else {
            "None".to_string()
        };
        generated.push_str(&format!("pub const {name}: Option<&'static [u8]> = {value};\n"));
    }
    fs::write(out_dir.join("hype_house_assets.rs"), generated)
        .expect("write generated house icon bindings");

    if env::var("CARGO_CFG_TARGET_OS").map(|v| v == "windows").unwrap_or(false) {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("ProductName", "Joker Tool");
        res.set("FileDescription", "Joker Tool");
        res.compile().unwrap();
    }
}
