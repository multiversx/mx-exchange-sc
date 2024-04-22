use clap::{Args, Parser, Subcommand};

/// Dex Interact CLI
#[derive(Default, PartialEq, Eq, Debug, Parser)]
#[command(version, about)]
#[command(propagate_version = true)]
pub struct InteractCli {
    #[command(subcommand)]
    pub command: Option<InteractCliCommand>,
}

/// Dex Interact CLI Commands
#[derive(Clone, PartialEq, Eq, Debug, Subcommand)]
pub enum InteractCliCommand {
    #[command(name = "pause", about = "Pause pair contract")]
    Pause,
    #[command(name = "swap", about = "Swaps token with a minimum return amount")]
    Swap(SwapArgs),
}

#[derive(Default, Clone, PartialEq, Eq, Debug, Args)]
pub struct SwapArgs {
    /// Amount to swap
    #[arg(short = 'a', long = "amount", verbatim_doc_comment)]
    pub amount: u128,
    /// The token id for the swap
    #[arg(short = 't', long = "token_identifier", verbatim_doc_comment)]
    pub token_identifier: String,
    /// Minimum return amount
    #[arg(short = 'm', long = "min_amount", verbatim_doc_comment)]
    pub min_amount: u128,
}
