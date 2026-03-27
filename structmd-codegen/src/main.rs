use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: structmd-codegen <schema.conf.md> <output.rs>");
        std::process::exit(2);
    }
    structmd_codegen::generate(&args[1], &args[2]);
    println!("generated: {}", args[2]);
}
