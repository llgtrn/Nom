/// Build script for nom-llvm.
///
/// Since we use `no-llvm-linking` in inkwell/llvm-sys (to avoid the
/// llvm-sys build script's restriction against dynamic linking on MSVC),
/// we handle LLVM linking ourselves by linking against LLVM-C.lib directly.
///
/// This also compiles the target initialization wrappers that llvm-sys
/// normally handles.
fn main() {
    // Find LLVM installation
    let llvm_dir = std::env::var("LLVM_SYS_180_PREFIX")
        .or_else(|_| std::env::var("LLVM_DIR"))
        .unwrap_or_else(|_| {
            // Default Windows LLVM installation path
            if cfg!(target_os = "windows") {
                "C:\\Program Files\\LLVM".to_string()
            } else {
                "/usr/lib/llvm-18".to_string()
            }
        });

    let llvm_lib_dir = format!("{}\\lib", llvm_dir);
    let llvm_include_dir = format!("{}\\include", llvm_dir);

    // Link against LLVM-C shared library (import lib on Windows)
    println!("cargo:rustc-link-search=native={}", llvm_lib_dir);
    println!("cargo:rustc-link-lib=dylib=LLVM-C");

    // Also check for the include dir in our local scripts prefix
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let local_include = format!("{}\\..\\..\\scripts\\llvm-prefix\\include", manifest_dir);

    // Compile the target initialization wrapper
    // This is needed because LLVM target init functions are inline in the C headers
    let include_dir =
        if std::path::Path::new(&format!("{}\\llvm-c\\Target.h", llvm_include_dir)).exists() {
            llvm_include_dir.clone()
        } else if std::path::Path::new(&format!("{}\\llvm-c\\Target.h", local_include)).exists() {
            local_include
        } else {
            // Fallback: skip target wrapper compilation
            println!("cargo:warning=LLVM-C headers not found, skipping target wrapper compilation");
            return;
        };

    cc::Build::new()
        .file("wrappers/target.c")
        .include(&include_dir)
        .compile("nom_llvm_wrappers");

    println!("cargo:rerun-if-changed=wrappers/target.c");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=LLVM_SYS_180_PREFIX");
    println!("cargo:rerun-if-env-changed=LLVM_DIR");
}
