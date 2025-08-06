use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

declare_id!("BHjZkKNQkAZX1t2zSXBLQaoSKN5U1zkthh9x2zq4odr2");

#[program]
pub mod insiders_job {
    use super::*;

    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(token_address: Pubkey, market_mint: Pubkey)]
pub struct InitializeMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = 8 + MarketConfig::INIT_SPACE,
        seeds = [b"config", market.key().as_ref()],
        bump,
    )]
    pub config: Account<'info, MarketConfig>,
    #[account(
        init,
        payer = admin,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market", market_mint.as_ref()], 
        bump,
    )]
    #[account(
        constraint = token_mint.key() == token_address
    )]
    pub token_mint: Account<'info, Mint>,
    #[account(
        constraint = round_mint.key() == market_mint
    )]
    pub round_mint: Account<'info, Mint>, // mint of the 24-hour market window
    pub market: Account<'info, Market>,

    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace, Debug)]
pub struct Market {
    pub admin: Pubkey,
    pub token_address: Pubkey,
    pub market_mint: Pubkey,
    pub start_ts: i64,
    pub end_ts: i64,
    pub total_stake: u64,
    pub total_score: Option<u64>,
    pub final_mcap: Option<u64>,
    pub resolved: bool,
}

#[account]
#[derive(InitSpace, Debug)]
pub struct MarketConfig {
    pub market: Pubkey,
    pub fee_rate: u64,
    pub min_stake: u64,
}
