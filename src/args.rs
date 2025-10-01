use clap::Parser;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(long, env)]
    pub persist_path: String,

    #[arg(long, env, default_value_t = 2)]
    pub fetch_interval: u64,
}
