fn main() {
    prost_build::compile_protos(
        &["../proto/messages.proto"],
        &["../proto/"],
    )
    .expect("Failed to compile protos");
}
