fn main() {
    prost_build::compile_protos(
        &[
            "../../proto/messages.proto",
            "../../proto/input.proto",
            "../../proto/file_transfer.proto",
            "../../proto/chat.proto",
        ],
        &["../../proto/"],
    )
    .expect("Failed to compile protos");
}
