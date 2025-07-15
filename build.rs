fn main() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    println!("cargo:warning=Writing to OUT_DIR = {}", out_dir);
    prost_build::compile_protos(
        &["./proto/nyks/module/v1/tx.proto"], // your message file
        &["./proto"],                             // include path
    ).expect("Failed to compile .proto files");
}