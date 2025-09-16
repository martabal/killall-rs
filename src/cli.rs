use clap::{
    Parser,
    builder::{Styles, styling::AnsiColor},
    command,
};

pub const MAX_NAMES: usize = std::mem::size_of::<usize>() * 8;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().bold())
    .usage(AnsiColor::Green.on_default().bold())
    .literal(AnsiColor::Cyan.on_default().bold())
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(Parser, Debug)]
#[command(styles = STYLES)]
#[command(author, version, about, long_about = None)]
pub struct FaulxArgs {
    /// process name to kill
    #[arg(required_unless_present = "list")]
    pub process_names: Vec<String>,

    /// list all known signal names
    #[arg(short = 'l', long)]
    pub list: bool,

    /// Send this signal instead of SIGTERM
    #[arg(short = 's', long)]
    pub signal: Option<String>,

    /// Don't print complaints
    #[arg(short = 'q', long)]
    pub quiet: bool,

    /// kill process group instead of process
    #[arg(short = 'g', long)]
    pub process_group: bool,
}
