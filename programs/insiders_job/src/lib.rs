use anchor_lang::prelude::*;
use anchor_spl::token::Mint;

declare_id!("BHjZkKNQkAZX1t2zSXBLQaoSKN5U1zkthh9x2zq4odr2");

#[program]
pub mod insiders_job {
    use super::*;

    pub fn initialize_config(
        // TODO: make sure its only callable by admin only
        ctx: Context<InitializeConfig>,
        fee_rate: u64,
        min_stake: u64,
    ) -> Result<()> {
        let config = &mut ctx.accounts.config;

        config.admin = ctx.accounts.admin.key();
        config.fee_rate = fee_rate;
        config.min_stake = min_stake;
        config.initialized = true;

        Ok(())
    }

    #[derive(Accounts)]
    pub struct InitializeConfig<'info> {
        #[account(mut)]
        admin: Signer<'info>,
        #[account(
            init,
            payer = admin,
            space = 8 + Config::INIT_SPACE,
            seeds = [b"config", ID.as_ref()],
            bump,
        )]
        pub config: Account<'info, Config>,
        pub system_program: Program<'info, System>,
    }

    #[account]
    #[derive(InitSpace, Debug)]
    pub struct Config {
        pub admin: Pubkey,
        pub fee_rate: u64,
        pub min_stake: u64,
        pub initialized: bool,
    }

    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        token_address: Pubkey,
        market_mint: Pubkey,
        start_ts: i64,
        end_ts: i64,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let config = &ctx.accounts.config;

        market.init(
            config.admin,
            token_address,
            market_mint,
            start_ts,
            end_ts,
            ctx.bumps.market,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(token_address: Pubkey, market_mint: Pubkey)]
pub struct InitializeMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        seeds = [b"config", ID.as_ref()],
        bump,
        constraint = config.initialized @ ErrorCode::ConfigNotInitialized,
        constraint = config.admin == admin.key() @ ErrorCode::Unathorized,
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = admin,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market", market_mint.as_ref()], 
        bump,
    )]
    pub market: Account<'info, Market>,

    #[account(
        constraint = token_mint.key() == token_address
    )]
    pub token_mint: Account<'info, Mint>,

    #[account(
        constraint = round_mint.key() == market_mint
    )]
    pub round_mint: Account<'info, Mint>, // mint of the 24-hour market window

    pub system_program: Program<'info, System>,
}

#[account]
#[derive(InitSpace, Debug)]
// Data account - holds the state
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
    pub bump: u8,
}

impl Market {
    pub fn init(
        &mut self,
        admin: Pubkey,
        token_address: Pubkey,
        market_mint: Pubkey,
        start_ts: i64,
        end_ts: i64,
        bump: u8,
    ) -> Result<()> {
        require!(end_ts > start_ts, ErrorCode::InvalidTimeRange);

        self.admin = admin;
        self.token_address = token_address;
        self.market_mint = market_mint;
        self.start_ts = start_ts;
        self.end_ts = end_ts;
        self.total_stake = 0;
        self.total_score = None;
        self.final_mcap = None;
        self.resolved = false;
        self.bump = bump;

        Ok(())
    }
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unathorized")]
    Unathorized,
    #[msg("Config not initializd")]
    ConfigNotInitialized,
    #[msg("Market end time must be after start time")]
    InvalidTimeRange,
    #[msg("Market has already ended")]
    MarketEnded,
    #[msg("Market has not started yet")]
    MarketNotStarted,
    #[msg("Market is already resolved")]
    AlreadyResolved,
    #[msg("Only admin can resolve market")]
    UnauthorizedResolution,
    #[msg("Stake amount below minimum")]
    StakeTooLow,
}
