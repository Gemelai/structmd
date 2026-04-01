fn main() {
    let schema = include_str!("schema/workflow.schema.md");
    let code = structmd_codegen::generate_from_text(schema, "workflow");
    let out_dir = std::env::var("OUT_DIR").unwrap();
    std::fs::write(format!("{}/workflow_config.rs", out_dir), code).unwrap();
    println!("cargo:rerun-if-changed=schema/workflow.schema.md");
}
