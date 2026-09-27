fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/orange.ico");
        if std::path::Path::new("assets/orange.ico").exists() {
            let mut res = winresource::WindowsResource::new();
            res.set_icon("assets/orange.ico");
            res.set("FileDescription", "Script Midair");
            res.set("ProductName", "Script Midair");
            if let Err(e) = res.compile() {
                println!("cargo:warning=icone non integree: {e}");
            }
        }
    }
}
