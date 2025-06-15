// programs/meme_launcher/src/lib.rs

use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token, TokenAccount, Mint},
    associated_token::AssociatedToken,
};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[program]
pub mod meme_launcher {
    use super::*;

    pub fn initialize_launch(
        ctx: Context<InitializeLaunch>,
        name: String,
        symbol: String,
        initial_supply: u64,
        curve_ratio: u64,
    ) -> Result<()> {
        let launch = &mut ctx.accounts.launch;
        launch.creator = ctx.accounts.creator.key();
        launch.mint = ctx.accounts.mint.key();
        launch.name = name;
        launch.symbol = symbol;
        launch.total_supply = initial_supply;
        launch.curve_ratio = curve_ratio;
        launch.is_active = true;
        
        Ok(())
    }

    pub fn buy_tokens(
        ctx: Context<BuyTokens>,
        amount: u64,
    ) -> Result<()> {
        require!(ctx.accounts.launch.is_active, ErrorCode::LaunchInactive);
        
        // Calculate price based on bonding curve
        let price = calculate_price(
            ctx.accounts.launch.total_supply,
            amount,
            ctx.accounts.launch.curve_ratio,
        )?;
        
        // Transfer SOL from buyer to vault
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.buyer.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                },
            ),
            price,
        )?;

        // Mint tokens to buyer
        anchor_spl::token::mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::MintTo {
                    mint: ctx.accounts.mint.to_account_info(),
                    to: ctx.accounts.buyer_token_account.to_account_info(),
                    authority: ctx.accounts.launch.to_account_info(),
                },
                &[&[
                    b"launch",
                    ctx.accounts.launch.creator.as_ref(),
                    &[ctx.bumps.launch],
                ]],
            ),
            amount,
        )?;

        ctx.accounts.launch.total_supply += amount;
        
        Ok(())
    }
}

#[derive(Accounts)]
pub struct InitializeLaunch<'info> {
    #[account(
        init,
        payer = creator,
        space = Launch::LEN,
        seeds = [b"launch", creator.key().as_ref()],
        bump
    )]
    pub launch: Account<'info, Launch>,
    
    #[account(mut)]
    pub creator: Signer<'info>,
    
    #[account(
        init,
        payer = creator,
        mint::decimals = 9,
        mint::authority = launch,
    )]
    pub mint: Account<'info, Mint>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
pub struct BuyTokens<'info> {
    #[account(
        mut,
        seeds = [b"launch", launch.creator.as_ref()],
        bump
    )]
    pub launch: Account<'info, Launch>,
    
    #[account(mut)]
    pub mint: Account<'info, Mint>,
    
    #[account(mut)]
    pub buyer: Signer<'info>,
    
    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = mint,
        associated_token::authority = buyer,
    )]
    pub buyer_token_account: Account<'info, TokenAccount>,
    
    #[account(mut)]
    /// CHECK: This is safe as it's just used as a vault
    pub vault: AccountInfo<'info>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

#[account]
pub struct Launch {
    pub creator: Pubkey,
    pub mint: Pubkey,
    pub name: String,
    pub symbol: String,
    pub total_supply: u64,
    pub curve_ratio: u64,
    pub is_active: bool,
}

impl Launch {
    pub const LEN: usize = 8 + 32 + 32 + 64 + 8 + 8 + 8 + 1;
}

#[error_code]
pub enum ErrorCode {
    #[msg("Launch is not active")]
    LaunchInactive,
}

// Helper function to calculate price based on bonding curve
fn calculate_price(current_supply: u64, amount: u64, curve_ratio: u64) -> Result<u64> {
    // Simple linear bonding curve: price = (current_supply + amount) * curve_ratio
    Ok((current_supply.checked_add(amount).unwrap())
        .checked_mul(curve_ratio)
        .unwrap())
}