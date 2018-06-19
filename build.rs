extern crate protobuf_codegen_pure;
extern crate protoc;
extern crate protoc_rust;

use protobuf_codegen_pure::Customize;

fn main() {
    // Ideally I would have combined these types but
    // it became a little late for the refactor so they are left as
    // discrete types.
    protoc_rust::run(protoc_rust::Args {
        out_dir: "src/public/proto",
        input: &[
            "proto/public/public.proto",
            "proto/public/request.proto",
            "proto/public/response.proto",
        ],
        includes: &["proto/public"],
        customize: Customize {
            ..Default::default()
        },
    }).expect("protoc");

    protoc_rust::run(protoc_rust::Args {
        out_dir: "src/server/proto",
        input: &["proto/db.proto"],
        includes: &["proto"],
        customize: Customize {
            ..Default::default()
        },
    }).expect("protoc");
}
