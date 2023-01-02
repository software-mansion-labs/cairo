use clap::Parser;
use utils::logging::init_logging;
use sierra_to_casm::compiler::{Args, compile_at_path};

fn main() {
    init_logging(log::LevelFilter::Off);
    log::info!("Starting Sierra compilation.");

    let args = Args::parse();

    compile_at_path(args).expect("compilation failed");
}
