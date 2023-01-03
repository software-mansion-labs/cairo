use clap::Parser;
use cairo_utils::logging::init_logging;
use cairo_sierra_to_casm::compiler::{Args, compile_at_path};

fn main() {
    init_logging(log::LevelFilter::Off);
    log::info!("Starting Sierra compilation.");

    let args = Args::parse();

    compile_at_path(&args.file).expect("compilation failed");
}
