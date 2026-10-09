use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

// Keep YOUR original declare_id! line if it differs.
// Verify with: solana address -k target/deploy/bonded_x402-keypair.json
declare_id!("CtXSSkVzvobPWuLZeciRcKa3fyXTUEgwmS1sUQnK9QZz");

/// BONDED x402
/// An API provider locks a USDC bond. Buyers pay per request (x402 style)
/// directly to the provider and get an on-chain receipt. If the provider
/// does not deliver, the buyer disputes within the window and the arbiter
/// can refund the buyer from the provider's bond.
#[program]
pub mod bonded_x402 {
    use super::*;

    /// Provider creates a bond vault and deposits `amount` of the bond mint.
    pub fn create_bond(
        ctx: Context<CreateBond>,
        arbiter: Pubkey,
        dispute_window: i64,
        amount: u64,
    ) -> Result<()> {
        require!(amount > 0, BondError::ZeroAmount);
        require!(dispute_window > 0, BondError::BadWindow);

        let bond = &mut ctx.accounts.bond;
        bond.provider = ctx.accounts.provider.key();
        bond.arbiter = arbiter;
        bond.mint = ctx.accounts.mint.key();
        bond.vault = ctx.accounts.vault.key();
        bond.exposure = 0;
        bond.dispute_window = dispute_window;
        bond.bump = ctx.bumps.bond;

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.provider_ata.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.provider.to_account_info(),
                },
            ),
            amount,
        )
    }

    /// Provider tops up the bond.
    pub fn top_up(ctx: Context<TopUp>, amount: u64) -> Result<()> {
        require!(amount > 0, BondError::ZeroAmount);
        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.provider_ata.to_account_info(),
                    to: ctx.accounts.vault.to_account_info(),
                    authority: ctx.accounts.provider.to_account_info(),
                },
            ),
            amount,
        )
    }

    /// Buyer pays the provider directly and gets a receipt.
    /// The payment must be fully covered by free (unexposed) bond.
    pub fn pay_and_access(ctx: Context<PayAndAccess>, nonce: u64, amount: u64) -> Result<()> {
        require!(amount > 0, BondError::ZeroAmount);

        let free = ctx
            .accounts
            .vault
            .amount
            .checked_sub(ctx.accounts.bond.exposure)
            .ok_or(BondError::MathError)?;
        require!(free >= amount, BondError::BondTooLow);

        token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.buyer_ata.to_account_info(),
                    to: ctx.accounts.provider_ata.to_account_info(),
                    authority: ctx.accounts.buyer.to_account_info(),
                },
            ),
            amount,
        )?;

        let now = Clock::get()?.unix_timestamp;
        let bond = &mut ctx.accounts.bond;
        bond.exposure = bond.exposure.checked_add(amount).ok_or(BondError::MathError)?;

        let receipt = &mut ctx.accounts.receipt;
        receipt.bond = bond.key();
        receipt.buyer = ctx.accounts.buyer.key();
        receipt.amount = amount;
        receipt.nonce = nonce;
        receipt.created_at = now;
        receipt.deadline = now.checked_add(bond.dispute_window).ok_or(BondError::MathError)?;
        receipt.status = ReceiptStatus::Active;
        receipt.bump = ctx.bumps.receipt;
        Ok(())
    }

    /// Buyer disputes a receipt before its deadline.
    pub fn dispute(ctx: Context<Dispute>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let receipt = &mut ctx.accounts.receipt;
        require!(receipt.status == ReceiptStatus::Active, BondError::WrongStatus);
        require!(now <= receipt.deadline, BondError::WindowClosed);
        receipt.status = ReceiptStatus::Disputed;
        Ok(())
    }

    /// Arbiter resolves a dispute. refund = true slashes the bond and
    /// refunds the buyer; false releases the bond lock.
    pub fn resolve(ctx: Context<Resolve>, refund: bool) -> Result<()> {
        require!(
            ctx.accounts.receipt.status == ReceiptStatus::Disputed,
            BondError::WrongStatus
        );
        let amount = ctx.accounts.receipt.amount;

        if refund {
            let provider = ctx.accounts.bond.provider;
            let bump = ctx.accounts.bond.bump;
            let seeds: &[&[u8]] = &[b"bond", provider.as_ref(), &[bump]];
            let signer = &[seeds];
            token::transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.vault.to_account_info(),
                        to: ctx.accounts.buyer_ata.to_account_info(),
                        authority: ctx.accounts.bond.to_account_info(),
                    },
                    signer,
                ),
                amount,
            )?;
            ctx.accounts.receipt.status = ReceiptStatus::Refunded;
        } else {
            ctx.accounts.receipt.status = ReceiptStatus::Released;
        }

        let bond = &mut ctx.accounts.bond;
        bond.exposure = bond.exposure.checked_sub(amount).ok_or(BondError::MathError)?;
        Ok(())
    }

    /// Anyone can release an undisputed receipt after its deadline.
    pub fn release(ctx: Context<Release>) -> Result<()> {
        let now = Clock::get()?.unix_timestamp;
        let receipt = &mut ctx.accounts.receipt;
        require!(receipt.status == ReceiptStatus::Active, BondError::WrongStatus);
        require!(now > receipt.deadline, BondError::WindowOpen);
        receipt.status = ReceiptStatus::Released;

        let bond = &mut ctx.accounts.bond;
        bond.exposure = bond
            .exposure
            .checked_sub(receipt.amount)
            .ok_or(BondError::MathError)?;
        Ok(())
    }

    /// Provider withdraws only the free (unexposed) part of the bond.
    pub fn withdraw_bond(ctx: Context<WithdrawBond>, amount: u64) -> Result<()> {
        let free = ctx
            .accounts
            .vault
            .amount
            .checked_sub(ctx.accounts.bond.exposure)
            .ok_or(BondError::MathError)?;
        require!(amount > 0 && free >= amount, BondError::BondTooLow);

        let provider = ctx.accounts.bond.provider;
        let bump = ctx.accounts.bond.bump;
        let seeds: &[&[u8]] = &[b"bond", provider.as_ref(), &[bump]];
        let signer = &[seeds];
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.vault.to_account_info(),
                    to: ctx.accounts.provider_ata.to_account_info(),
                    authority: ctx.accounts.bond.to_account_info(),
                },
                signer,
            ),
            amount,
        )
    }
}

// ---------- State ----------

#[account]
#[derive(InitSpace)]
pub struct Bond {
    pub provider: Pubkey,
    pub arbiter: Pubkey,
    pub mint: Pubkey,
    pub vault: Pubkey,
    pub exposure: u64,
    pub dispute_window: i64,
    pub bump: u8,
}

#[account]
#[derive(InitSpace)]
pub struct Receipt {
    pub bond: Pubkey,
    pub buyer: Pubkey,
    pub amount: u64,
    pub nonce: u64,
    pub created_at: i64,
    pub deadline: i64,
    pub status: ReceiptStatus,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, InitSpace)]
pub enum ReceiptStatus {
    Active,
    Disputed,
    Refunded,
    Released,
}

// ---------- Accounts ----------

#[derive(Accounts)]
pub struct CreateBond<'info> {
    #[account(mut)]
    pub provider: Signer<'info>,
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = provider,
        space = 8 + Bond::INIT_SPACE,
        seeds = [b"bond", provider.key().as_ref()],
        bump
    )]
    pub bond: Account<'info, Bond>,
    #[account(
        init,
        payer = provider,
        seeds = [b"vault", bond.key().as_ref()],
        bump,
        token::mint = mint,
        token::authority = bond
    )]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, token::mint = mint, token::authority = provider)]
    pub provider_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct TopUp<'info> {
    pub provider: Signer<'info>,
    #[account(has_one = provider)]
    pub bond: Account<'info, Bond>,
    #[account(mut, address = bond.vault @ BondError::BadVault)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, token::mint = bond.mint, token::authority = provider)]
    pub provider_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(nonce: u64)]
pub struct PayAndAccess<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    #[account(mut)]
    pub bond: Account<'info, Bond>,
    #[account(address = bond.vault @ BondError::BadVault)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, token::mint = bond.mint, token::authority = buyer)]
    pub buyer_ata: Account<'info, TokenAccount>,
    #[account(mut, token::mint = bond.mint, token::authority = bond.provider)]
    pub provider_ata: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = buyer,
        space = 8 + Receipt::INIT_SPACE,
        seeds = [b"receipt", bond.key().as_ref(), buyer.key().as_ref(), &nonce.to_le_bytes()],
        bump
    )]
    pub receipt: Account<'info, Receipt>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Dispute<'info> {
    pub buyer: Signer<'info>,
    pub bond: Account<'info, Bond>,
    #[account(mut, has_one = buyer, has_one = bond)]
    pub receipt: Account<'info, Receipt>,
}

#[derive(Accounts)]
pub struct Resolve<'info> {
    pub arbiter: Signer<'info>,
    #[account(mut, has_one = arbiter)]
    pub bond: Account<'info, Bond>,
    #[account(mut, address = bond.vault @ BondError::BadVault)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, has_one = bond)]
    pub receipt: Account<'info, Receipt>,
    #[account(mut, token::mint = bond.mint, token::authority = receipt.buyer)]
    pub buyer_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Release<'info> {
    pub caller: Signer<'info>,
    #[account(mut)]
    pub bond: Account<'info, Bond>,
    #[account(mut, has_one = bond)]
    pub receipt: Account<'info, Receipt>,
}

#[derive(Accounts)]
pub struct WithdrawBond<'info> {
    pub provider: Signer<'info>,
    #[account(has_one = provider)]
    pub bond: Account<'info, Bond>,
    #[account(mut, address = bond.vault @ BondError::BadVault)]
    pub vault: Account<'info, TokenAccount>,
    #[account(mut, token::mint = bond.mint, token::authority = provider)]
    pub provider_ata: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>,
}

// ---------- Errors ----------

#[error_code]
pub enum BondError {
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Dispute window must be positive")]
    BadWindow,
    #[msg("Free bond is too low")]
    BondTooLow,
    #[msg("Vault does not match bond")]
    BadVault,
    #[msg("Receipt is in the wrong status")]
    WrongStatus,
    #[msg("Dispute window already closed")]
    WindowClosed,
    #[msg("Dispute window still open")]
    WindowOpen,
    #[msg("Math overflow")]
    MathError,
}

