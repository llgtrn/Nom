fn main() {
    let exe_path = std::env::current_exe().unwrap();
    let prefix = exe_path.parent().unwrap().parent().unwrap();
    let prefix_str = prefix.display().to_string();

    let llvm_lib_dir = "C:\\Program Files\\LLVM\\lib";

    let args: Vec<String> = std::env::args().collect();
    let arg = args.get(1).map(|s| s.as_str()).unwrap_or("");
    match arg {
        "--version" => println!("18.1.8"),
        "--prefix" => println!("{}", prefix_str),
        "--libdir" => println!("{}", llvm_lib_dir),
        "--includedir" => println!("{}\\include", prefix_str),
        "--cflags" => println!("-I{}\\include", prefix_str),
        "--cxxflags" => println!("-I{}\\include", prefix_str),
        "--ldflags" => println!("-L{}", llvm_lib_dir),
        "--system-libs" => println!(""),
        "--libs" => println!("-lLLVM-C"),
        "--shared-mode" => println!("shared"),
        "--link-shared" => println!(""),
        "--link-static" => println!(""),
        "--has-rtti" => println!("YES"),
        "--assertion-mode" => println!("OFF"),
        "--components" => println!("all"),
        "--targets-built" => println!("AArch64 AMDGPU ARM AVR BPF Hexagon Lanai LoongArch Mips MSP430 NVPTX PowerPC RISCV Sparc SystemZ VE WebAssembly X86 XCore"),
        "--host-target" => println!("x86_64-pc-windows-msvc"),
        "--build-mode" => println!("Release"),
        _ => {}
    }
}
