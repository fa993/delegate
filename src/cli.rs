use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct CLIArgs {
    /// Kills a delegate command with matching name/pid
    #[arg(short, long, value_name = "PID")]
    pub(crate) kill: Option<String>,

    /// subcommands
    #[command(subcommand)]
    pub(crate) subcommand: Option<SubCommand>,

    /// Command to delegate execution
    pub(crate) delegate: Vec<String>,
}

#[derive(Subcommand)]
pub enum SubCommand {
    /// Lists ongoing executions
    List,

    /// kills executions if any and deletes everything
    Reset,
}
