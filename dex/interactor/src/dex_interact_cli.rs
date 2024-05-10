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
    #[command(name = "swap", about = "Swaps token with a minimum return amount")]
    Swap(SwapArgs),
    #[command(name = "add_liquidity", about = "Adds liquidity to a pair")]
    Add(AddArgs),
    #[command(name = "full_farm", about = "Creates a full farm scenario")]
    FullFarm(AddArgs)
}

// Second token id is taken from the state
#[derive(Default, Clone, PartialEq, Eq, Debug, Args)]
pub struct SwapArgs {
    /// Amount to swap
    #[arg(short = 'a', long = "amount", verbatim_doc_comment)]
    pub amount: u128,
    /// Minimum return amount
    #[arg(short = 'm', long = "min_amount", verbatim_doc_comment)]
    pub min_amount: u128,
}

#[derive(Default, Clone, PartialEq, Eq, Debug, Args)]
pub struct AddArgs {
    /// Amount to send from first token
    #[arg(long = "fist_amount", verbatim_doc_comment)]
    pub first_payment_amount: u128,
    /// Amount to send from second token
    #[arg(long = "second_amount", verbatim_doc_comment)]
    pub second_payment_amount: u128,
    /// Min amount accepted for first token (slippage)
    #[arg(long = "first_amount_min", verbatim_doc_comment)]
    pub first_token_amount_min: u128,
    /// Min amount accepted for second token (slippage)
    #[arg(long = "second_amount_min", verbatim_doc_comment)]
    pub second_token_amount_min: u128,
}
