fn main() {
    // Expose build date as BUILD_DATE env var for compile-time version string
    let output = std::process::Command::new("date")
        .args(["-u", "+%Y-%m-%d"])
        .output()
        .expect("failed to run date command");
    let date = String::from_utf8(output.stdout)
        .expect("invalid utf8 from date")
        .trim()
        .to_string();
    println!("cargo:rustc-env=BUILD_DATE={date}");

    let out_dir = std::env::var("OUT_DIR").unwrap();
    println!("cargo:warning=Writing to OUT_DIR = {}", out_dir);
    prost_build::compile_protos(
        &["./proto/nyks/bridge/tx.proto", "./proto/nyks/zkos/tx.proto"], // your message file
        &["./proto"],                                                    // include path
    )
    .expect("Failed to compile .proto files");
}
