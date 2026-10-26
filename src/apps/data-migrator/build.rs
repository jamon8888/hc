fn main() {
    for name in [
        "ALEX_RELEASE_CHANNEL",
        "ALEX_PRODUCT_ID",
        "ALEX_DESKTOP_BINARY_NAME",
        "ALEX_DATA_MIGRATOR_BINARY_NAME",
    ] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    tauri_build::build();
}
