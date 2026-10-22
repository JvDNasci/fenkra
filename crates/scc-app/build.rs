//! Embute o ícone da fênix no executável (Windows).
fn main() {
    println!("cargo:rerun-if-changed=../../assets/app/fenix.ico");
    #[cfg(windows)]
    {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/app/fenix.ico");
        res.set("ProductName", "SolidCadCraft");
        res.set("FileDescription", "SolidCadCraft — CAD paramétrico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=não foi possível embutir o ícone: {e}");
        }
    }
}
