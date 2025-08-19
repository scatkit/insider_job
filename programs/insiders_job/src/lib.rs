use anchor_lang::{prelude::*, solana_program::program::invoke_signed};
use anchor_spl::{associated_token::AssociatedToken, token::{Mint, SetAuthority, Token, TokenAccount}, token_2022::Token2022};
 

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

/// USER: PLACE PREDICITON
pub fn place_prediction(
    ctx: Context<PlacePredictionCtx>,
    guessed_mcap: u64,
    stake: u64,
) -> Result<()>{
    let market = &mut ctx.accounts.market;
    let user = &mut ctx.accounts.user;
    let config = &ctx.accounts.config;
    
    let now = Clock::get()?.unix_timestamp;
    require!(now >= market.start_ts, MarketErrorCode::MarketNotStarted);
    require!(now < market.end_ts, MarketErrorCode::MarketEnded);
    require!(!market.resolved, MarketErrorCode::AlreadyResolved);
    
    require!(stake >= config.min_stake, MarketErrorCode::StakeTooLow);
    
    // Transfering user's SOL to the market reserve
    let cpi_context = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            anchor_lang::system_program::Transfer{
                from: user.to_account_info(),
                to: market.to_account_info(),
            },
        );
    anchor_lang::system_program::transfer(cpi_context, stake)?;
    
    // Update the market state
    market.sol_reserve = market.sol_reserve.checked_add(stake).ok_or(MarketErrorCode::CalculationOverflow)?;
    
    let prediction_data = &mut ctx.accounts.prediction_data;
    prediction_data.market = market.key();
    prediction_data.prediction_mint = ctx.accounts.prediction_token_mint.key();
    prediction_data.guessed_mcap = guessed_mcap;
    prediction_data.stake = stake;
    prediction_data.timestamp = now;
    prediction_data.bump = ctx.bumps.prediction_data;
    
    let market_bump = market.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[
        b"market",
        market.market_mint.as_ref(),
        &[market_bump],
    ]];
        
        let cpi_accounts =anchor_spl::token::MintTo{
            mint: ctx.accounts.prediction_token_mint.to_account_info(),
            to: ctx.accounts.user_bet_token_account.to_account_info(),
            authority: market.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);
        anchor_spl::token::mint_to(cpi_ctx, 1)?;
    
    // Revoking mint authority
    anchor_spl::token::set_authority(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            SetAuthority{
                account_or_mint: ctx.accounts.prediction_token_mint.to_account_info(),
                current_authority: market.to_account_info(),
            },
        signer_seeds,
    ),
    anchor_spl::token::spl_token::instruction::AuthorityType::MintTokens,
    None,
    )?;
    
        // Revoking mint authority
    anchor_spl::token::set_authority(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            SetAuthority{
                account_or_mint: ctx.accounts.prediction_token_mint.to_account_info(),
                current_authority: market.to_account_info(),
            },
        signer_seeds,
    ),
    anchor_spl::token::spl_token::instruction::AuthorityType::FreezeAccount,
    None,
    )?;
     
    Ok(())
}


#[derive(Accounts)]
pub struct PlacePredictionCtx<'info>{
    #[account(mut)]
    pub user: Signer<'info>,
    
    /// CHECK: just used for PDA derivation
    pub market_mint: AccountInfo<'info>,
    
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
    
    #[account(
        init,
        payer = user,
        mint::decimals = 0,
        mint::authority = market,
        mint::freeze_authority = market,
    )]
    pub prediction_token_mint: Account<'info, Mint>, // pased by the user
        
    #[account(
        init,
        payer = user,
        space = 8 + PredictionData::INIT_SPACE,
        seeds = [b"prediction", prediction_token_mint.key().as_ref()],
        bump,
    )]
    pub prediction_data: Account<'info, PredictionData>,
    
    #[account(
        init,
        payer = user,
        associated_token::mint = prediction_token_mint,
        associated_token::authority = user,
    )]
    pub user_bet_token_account: Account<'info, TokenAccount>,
    
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
    
    #[account]
    #[derive(InitSpace, Debug)]
    pub struct PredictionData {
        pub market: Pubkey,
        pub prediction_mint: Pubkey,
        pub guessed_mcap: u64,
        pub stake: u64,
        pub timestamp: i64,
        pub bump: u8,
}
