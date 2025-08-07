use anchor_lang::prelude::*;
use anchor_spl::{associated_token::AssociatedToken, token::{Mint, Token, TokenAccount}};

declare_id!("BHjZkKNQkAZX1t2zSXBLQaoSKN5U1zkthh9x2zq4odr2");

#[program]
pub mod insiders_job {
    use super::*;
    #[error_code]
    pub enum MarketErrorCode {
        #[msg("Unauthorized")]
        Unauthorized,
        #[msg("Config not initialized")]
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
        #[msg("Fee too high")]
        FeeTooHigh,
        #[msg("Calculation overflow")]
        CalculationOverflow,
    }
    
    /// ADMIN: INITIALIZE/UPDATE CONFIG:
    
    const MAX_FEE_RATE_BPS: u64 = 1000; // 10%
    const MIN_STAKE_LAMPORTS: u64 = 40_000_000; // 0.04 sol
    
    pub fn initialize_config(
        ctx: Context<InitializeConfig>,
        fee_rate: u64,
        min_stake: u64,
    ) -> Result<()> {
        // TODO: add the admin check
        require!(fee_rate <= MAX_FEE_RATE_BPS, MarketErrorCode::FeeTooHigh);
        require!(
            min_stake >= MIN_STAKE_LAMPORTS,
            MarketErrorCode::StakeTooLow
        );

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
    
    pub fn update_config(
        ctx: Context<UpdateConfig>,
        fee_rate_bps: Option<u64>,
        min_stake_lamports: Option<u64>,
    ) -> Result<()>{
        let config = &mut ctx.accounts.config;
        
        if let Some(fee_rate) = fee_rate_bps{
            require!(fee_rate <= MAX_FEE_RATE_BPS, MarketErrorCode::FeeTooHigh);
            config.fee_rate = fee_rate;
        }
        
        if let Some(min_stake) = min_stake_lamports{
            require!(min_stake >= MIN_STAKE_LAMPORTS, MarketErrorCode::StakeTooLow);
            config.min_stake = min_stake;
        }
        
        Ok(())
    }

    #[derive(Accounts)]
    pub struct UpdateConfig<'info> {
        #[account(mut)]
        pub admin: Signer<'info>,
        #[account(
            mut, 
            seeds = [b"config", ID.as_ref()],
            bump, 
            constraint = config.admin == admin.key() @ MarketErrorCode::Unauthorized,
        )]
        pub config: Account<'info, Config>,
    }

    #[account]
    #[derive(InitSpace, Debug)]
    pub struct Config {
        pub admin: Pubkey,
        pub fee_rate: u64,
        pub min_stake: u64,
        pub initialized: bool,
    }

    /// ADMIN: INITIALIZE MARKET 
    pub fn initialize_market(
        ctx: Context<InitializeMarket>,
        token_address: Pubkey,
        market_mint: Pubkey,
        duration_seconds: i64,
    ) -> Result<()> {
        let market = &mut ctx.accounts.market;
        let config = &ctx.accounts.config;
        
        let now = Clock::get()?.unix_timestamp;
        let start_ts = now;
        let end_ts = now + duration_seconds;
        
        let market_init_args = MarketInitArgs {
            admin: config.admin,
            token_address,
            market_mint,
            start_ts,
            end_ts,
            bump: ctx.bumps.market,
        };

        market.init(market_init_args)?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(token_address: Pubkey)]
pub struct InitializeMarket<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    
    #[account(
        init,
        payer = admin,
        mint::decimals = 0,
        mint::authority = market,
        mint::freeze_authority = market,
    )]
    pub market_mint: Account<'info, Mint>, // mint of the 24-hour market window
    #[account(
        seeds = [b"config", ID.as_ref()],
        bump,
        constraint = config.initialized @ MarketErrorCode::ConfigNotInitialized,
        constraint = config.admin == admin.key() @ MarketErrorCode::Unauthorized,
    )]
    pub config: Account<'info, Config>,

    #[account(
        init,
        payer = admin,
        space = 8 + Market::INIT_SPACE,
        seeds = [b"market", market_mint.key().as_ref()], // TODO: is it the right seed?
        bump,
    )]
    pub market: Account<'info, Market>,

    #[account(
        constraint = token_mint.key() == token_address
    )]
    pub token_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
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
    pub sol_reserve: u64,
    pub total_score: Option<u64>,
    pub final_mcap: Option<u64>,
    pub resolved: bool,
    pub bump: u8,
}

pub struct MarketInitArgs {
    pub admin: Pubkey,
    pub token_address: Pubkey,
    pub market_mint: Pubkey,
    pub start_ts: i64,
    pub end_ts: i64,
    pub bump: u8,
}

impl Market {
    pub fn init(
        &mut self,
        args: MarketInitArgs,
    ) -> Result<()> {
        require!(args.end_ts > args.start_ts, MarketErrorCode::InvalidTimeRange);

        self.admin = args.admin;
        self.token_address = args.token_address;
        self.market_mint = args.market_mint;
        self.start_ts = args.start_ts;
        self.end_ts = args.end_ts;
        self.bump = args.bump;
        self.sol_reserve = 0;
        self.total_score = None;
        self.final_mcap = None;
        self.resolved = false;

        Ok(())
    }
}

pub fn place_prediction(
    ctx: Context<PlacePredictionCtx>,
    guessed_mcap: u64,
    stake: u64,
) -> Result<()>{
    let market = &mut ctx.accounts.market;
    let config = &ctx.accounts.config;
    
    let now = Clock::get()?.unix_timestamp;
    require!(now >= market.start_ts, MarketErrorCode::MarketNotStarted);
    require!(now < market.end_ts, MarketErrorCode::MarketEnded);
    require!(!market.resolved, MarketErrorCode::AlreadyResolved);
    
    require!(stake >= config.min_stake, MarketErrorCode::StakeTooLow);
    
    // Calculate the score
    let user_bet = stake.checked_mul(guessed_mcap).ok_or(MarketErrorCode::CalculationOverflow);
    
    // Transfering user's SOL to the market reserve
    let user_address = ctx.accounts.user.to_account_info(); 
    let market_address = ctx.accounts.market.to_account_info();
    
    anchor_lang::system_program::transfer(
        CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer{
                from: user_address.clone(),
                to: market_address.clone(),
            }
        ),
        stake,
    )?;
    
    // market.sol_reserve = market.sol_reserve.checked_add(stake).ok_or(MarketErrorCode::CalculationOverflow)?;
    
    
    Ok(())
}

#[derive(Accounts)]
pub struct PlacePredictionCtx<'info>{
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        mut,
        seeds = [b"market", market_mint.key().as_ref()],
        bump = market.bump,
        
    )]
    pub market: Account<'info, Market>,
     
    #[account(
        seeds = [b"config", ID.as_ref()],
        bump,
    )]
    pub config: Account<'info, Config>,
    
    #[account(mut)] 
    pub market_mint: Account<'info, Mint>,
    
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = market_mint,
        associated_token::authority = user,
    )]
    pub user_bet_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Account<'info, TokenAccount>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
