use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use metaplex_token_metadata::state::Metadata;
use std::convert::TryInto;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod constants {
    pub const STAKING_PDA_SEED: &[u8] = b"nft_staking";
}

#[program]
pub mod nft_staking {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>, _nonce_staking: u8) -> ProgramResult {
        ctx.accounts.staking_account.initializer_key = *ctx.accounts.initializer.key;

        Ok(())
    }

    pub fn stake(ctx: Context<Stake>, _nonce_staking: u8) -> ProgramResult {
        let token_metadata = &ctx.accounts.token_metadata;
        let metadata = Metadata::from_account_info(&token_metadata)?;

        match metadata.data.creators {
            Some(creators) => {
                for creator in creators {
                    assert_keys_equal(
                        creator.address,
                        ctx.accounts.staking_account.initializer_key,
                    )?;
                }
            }
            None => {
                msg!("No creators found in metadata");
            }
        }
        Ok(())
    }
}

pub fn assert_keys_equal(key1: Pubkey, key2: Pubkey) -> ProgramResult {
    if key1 != key2 {
        Err(ErrorCode::PublicKeyMismatch.into())
    } else {
        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct Initialize<'info> {
    #[account(mut)]
    ///pays rent on the initializing accounts
    pub initializer: Signer<'info>,

    #[account(
        init,
        payer = initializer,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: ProgramAccount<'info, StakingAccount>,

    ///used by anchor for init of the token
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct Stake<'info> {
    //the authority allowed to transfer from token_from
    pub token_from_authority: Signer<'info>,

    pub token_metadata: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
        constraint = !staking_account.freeze_program,
    )]
    pub staking_account: ProgramAccount<'info, StakingAccount>,
}

#[account]
#[derive(Default)]
pub struct StakingAccount {
    pub initializer_key: Pubkey,
    pub freeze_program: bool,
}

#[error]
pub enum ErrorCode {
    #[msg("PublicKeyMismatch")]
    PublicKeyMismatch,
}
