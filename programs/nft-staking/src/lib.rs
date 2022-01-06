pub mod utils;

use crate::utils::{
    spl_close_account, spl_token_mint, spl_token_transfer, CloseAccountParams, TokenMintParams,
    TokenTransferParams,
};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};
use metaplex_token_metadata::state::Metadata;
use spl_token::instruction::AuthorityType;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[cfg(not(feature = "local-testing"))]
pub mod constants {
    pub const DAO_TOKEN_MINT_PUBKEY: &str = "AURYydfxJib1ZkTir1Jn1J9ECYUtjb6rKQVmtYaixWPP";
    pub const STAKING_PDA_SEED: &[u8] = b"nft_staking";
}

#[cfg(feature = "local-testing")]
pub mod constants {
    pub const DAO_TOKEN_MINT_PUBKEY: &str = "teST1ieLrLdr4MJPZ7i8mgSCLQ7rTrPRjNnyFdHFaz9";
    pub const STAKING_PDA_SEED: &[u8] = b"staking";
}

#[program]
pub mod nft_staking {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        _nonce_staking: u8,
        authorized_creator: Pubkey,
        authorized_name_starts: Vec<String>,
    ) -> ProgramResult {
        ctx.accounts.staking_account.admin_key = *ctx.accounts.initializer.key;
        ctx.accounts.staking_account.authorized_creator = authorized_creator;
        ctx.accounts
            .staking_account
            .authorized_name_starts
            .extend(authorized_name_starts);

        Ok(())
    }

    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn toggle_freeze_program(ctx: Context<FreezeProgram>, _nonce_staking: u8) -> ProgramResult {
        ctx.accounts.staking_account.freeze_program = !ctx.accounts.staking_account.freeze_program;

        Ok(())
    }

    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn update_admin(
        ctx: Context<UpdateAdmin>,
        _nonce_staking: u8,
        new_admin: Pubkey,
    ) -> ProgramResult {
        ctx.accounts.staking_account.admin_key = new_admin;

        Ok(())
    }

    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn update_authorized_creator(
        ctx: Context<UpdateAuthorizedCreator>,
        _nonce_staking: u8,
        new_authorized_creator: Pubkey,
    ) -> ProgramResult {
        ctx.accounts.staking_account.authorized_creator = new_authorized_creator;

        Ok(())
    }

    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn add_authorized_name_starts(
        ctx: Context<UpdateAuthorizedCreator>,
        _nonce_staking: u8,
        new_authorized_name_starts: Vec<String>,
    ) -> ProgramResult {
        for new_authorized_name_start in new_authorized_name_starts.iter() {
            if ctx
                .accounts
                .staking_account
                .authorized_name_starts
                .iter()
                .find(|&authorized_name_start| authorized_name_start == new_authorized_name_start)
                == None
            {
                ctx.accounts
                    .staking_account
                    .authorized_name_starts
                    .push(new_authorized_name_start.to_string());
            }
        }

        Ok(())
    }

    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn remove_authorized_name_starts(
        ctx: Context<RemoveAuthorizedNameStarts>,
        _nonce_staking: u8,
        old_authorized_name_starts: Vec<String>,
    ) -> ProgramResult {
        for old_authorized_name_start in old_authorized_name_starts.iter() {
            match ctx
                .accounts
                .staking_account
                .authorized_name_starts
                .iter()
                .position(|authorized_name_start| {
                    authorized_name_start == old_authorized_name_start
                }) {
                Some(index) => {
                    ctx.accounts
                        .staking_account
                        .authorized_name_starts
                        .remove(index);
                }
                None => {}
            }
        }

        Ok(())
    }

    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn add_reward(
        ctx: Context<AddReward>,
        _nonce_staking: u8,
        nft_mint_keys: Vec<Pubkey>,
    ) -> ProgramResult {
        for nft_mint_key in nft_mint_keys.iter() {
            if ctx
                .accounts
                .staking_account
                .active_rewards
                .iter()
                .find(|&active_reward| active_reward == nft_mint_key)
                == None
            {
                // add active reward
                ctx.accounts
                    .staking_account
                    .active_rewards
                    .push(*nft_mint_key);
            }
        }

        Ok(())
    }

    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn remove_reward(ctx: Context<RemoveReward>, nonce_staking: u8) -> ProgramResult {
        match ctx
            .accounts
            .staking_account
            .active_rewards
            .iter()
            .position(|active_reward| active_reward == ctx.accounts.nft_mint.key)
        {
            Some(index) => {
                // remove active reward
                ctx.accounts.staking_account.active_rewards.remove(index);

                // compute staking_account signer seeds
                let seeds = &[constants::STAKING_PDA_SEED.as_ref(), &[nonce_staking]];
                let signer = [&seeds[..]];

                // transfer nft mint authority
                let cpi_ctx = CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    token::SetAuthority {
                        current_authority: ctx.accounts.staking_account.to_account_info(),
                        account_or_mint: ctx.accounts.nft_mint.to_account_info(),
                    },
                    &signer,
                );
                token::set_authority(
                    cpi_ctx,
                    AuthorityType::MintTokens,
                    Some(*ctx.accounts.nft_mint_authority_to.key),
                )?;

                return Ok(());
            }
            None => {
                return Err(ErrorCode::InvalidMintForReward.into());
            }
        }
    }

    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn add_winner(
        ctx: Context<AddWinner>,
        _nonce_staking: u8,
        _nonce_user_staking: u8,
        _winner: Pubkey,
    ) -> ProgramResult {
        // Check if nft is one of the rewards
        if ctx
            .accounts
            .staking_account
            .active_rewards
            .iter()
            .find(|&active_reward| active_reward == ctx.accounts.nft_mint.key)
            == None
        {
            return Err(ErrorCode::InvalidMintForReward.into());
        }

        match ctx
            .accounts
            .user_staking_account
            .claimable
            .iter()
            .position(|claimable_token| claimable_token.nft_mint == *ctx.accounts.nft_mint.key)
        {
            Some(index) => {
                ctx.accounts.user_staking_account.claimable[index].amount += 1;
            }
            None => {
                ctx.accounts
                    .user_staking_account
                    .claimable
                    .push(ClaimableToken {
                        nft_mint: *ctx.accounts.nft_mint.key,
                        amount: 1,
                    });
            }
        }

        Ok(())
    }

    pub fn stake(
        ctx: Context<Stake>,
        _nonce_nft_vault: u8,
        _nonce_staking: u8,
        _nonce_user_staking: u8,
    ) -> ProgramResult {
        let nft_metadata = &ctx.accounts.nft_metadata;
        let metadata = Metadata::from_account_info(&nft_metadata)?;

        // determine authorized name start
        if ctx
            .accounts
            .staking_account
            .authorized_name_starts
            .iter()
            .find(|&authorized_name_start| {
                metadata
                    .data
                    .name
                    .starts_with(&authorized_name_start.to_string())
            })
            == None
        {
            return Err(ErrorCode::NoAuthorizedNameStartFoundInMetadata.into());
        }

        match metadata.data.creators {
            Some(creators) => {
                // determine authorized creator
                if creators.iter().find(|&creator| {
                    creator.verified
                        && creator.address == ctx.accounts.staking_account.authorized_creator
                }) == None
                {
                    return Err(ErrorCode::NoAuthorizedCreatorsFoundInMetadata.into());
                }

                // set user staking wallet
                ctx.accounts.user_staking_account.wallet = *ctx.accounts.nft_from_authority.key;

                // transfer nft to nft vault
                spl_token_transfer(TokenTransferParams {
                    source: ctx.accounts.nft_from.to_account_info(),
                    destination: ctx.accounts.nft_vault.to_account_info(),
                    authority: ctx.accounts.nft_from_authority.to_account_info(),
                    authority_signer_seeds: &[],
                    token_program: ctx.accounts.token_program.to_account_info(),
                    amount: 1,
                })?;

                // push nft_mint_key from the nft_mint_keys
                ctx.accounts
                    .user_staking_account
                    .nft_mint_keys
                    .push(*ctx.accounts.nft_mint.key);

                return Ok(());
            }
            None => {
                return Err(ErrorCode::NoAuthorizedCreatorsFoundInMetadata.into());
            }
        };
    }

    pub fn unstake(
        ctx: Context<Unstake>,
        _nonce_nft_vault: u8,
        nonce_staking: u8,
        _nonce_user_staking: u8,
    ) -> ProgramResult {
        if ctx.accounts.user_staking_account.claimable.len() > 0 {
            return Err(ErrorCode::CantUnstakeBeforeClaim.into());
        }

        match ctx
            .accounts
            .user_staking_account
            .nft_mint_keys
            .iter()
            .position(|nft_mint_key| nft_mint_key == ctx.accounts.nft_mint.key)
        {
            Some(index) => {
                // compute staking account signer seeds
                let staking_account_seeds =
                    &[constants::STAKING_PDA_SEED.as_ref(), &[nonce_staking]];
                let staking_account_signer = &staking_account_seeds[..];

                // transfer nft to user
                spl_token_transfer(TokenTransferParams {
                    source: ctx.accounts.nft_vault.to_account_info(),
                    destination: ctx.accounts.nft_to.to_account_info(),
                    authority: ctx.accounts.staking_account.to_account_info(),
                    authority_signer_seeds: staking_account_signer,
                    token_program: ctx.accounts.token_program.to_account_info(),
                    amount: 1,
                })?;

                // pop nft_mint_key from the nft_mint_keys
                ctx.accounts
                    .user_staking_account
                    .nft_mint_keys
                    .remove(index);

                // Close nft_vault tokenAccount
                spl_close_account(CloseAccountParams {
                    account: ctx.accounts.nft_vault.to_account_info(),
                    destination: ctx.accounts.nft_to_authority.to_account_info(),
                    owner: ctx.accounts.staking_account.to_account_info(),
                    owner_signer_seeds: staking_account_signer,
                    token_program: ctx.accounts.token_program.to_account_info(),
                })?;

                return Ok(());
            }
            None => {
                return Err(ErrorCode::NotStakedItem.into());
            }
        }
    }

    pub fn claim(ctx: Context<Claim>, nonce_staking: u8, _nonce_user_staking: u8) -> ProgramResult {
        match ctx
            .accounts
            .user_staking_account
            .claimable
            .iter()
            .position(|claimable_token| claimable_token.nft_mint == *ctx.accounts.nft_mint.key)
        {
            Some(index) => {
                let claimable_token = ctx.accounts.user_staking_account.claimable[index];

                // remove claimed item from user
                ctx.accounts.user_staking_account.claimable.remove(index);

                // check if claim token is active reward
                if ctx
                    .accounts
                    .staking_account
                    .active_rewards
                    .iter()
                    .find(|&&active_reward| active_reward == claimable_token.nft_mint)
                    == None
                {
                    return Ok(());
                }

                // compute staking account signer seeds
                let staking_account_seeds =
                    &[constants::STAKING_PDA_SEED.as_ref(), &[nonce_staking]];
                let staking_account_signer = &staking_account_seeds[..];

                // mint claimable amounts to user
                spl_token_mint(TokenMintParams {
                    mint: ctx.accounts.nft_mint.to_account_info(),
                    to: ctx.accounts.nft_to.to_account_info(),
                    amount: claimable_token.amount as u64,
                    owner: ctx.accounts.staking_account.to_account_info(),
                    owner_signer_seeds: staking_account_signer,
                    token_program: ctx.accounts.token_program.to_account_info(),
                })?;

                return Ok(());
            }
            None => {
                return Err(ErrorCode::NotClaimableItem.into());
            }
        }
    }

    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn mint_to(ctx: Context<MintTo>, nonce_staking: u8, amount: u64) -> ProgramResult {
        if ctx
            .accounts
            .staking_account
            .active_rewards
            .iter()
            .find(|&active_reward| active_reward == ctx.accounts.nft_mint.key)
            == None
        {
            return Err(ErrorCode::InvalidMintForReward.into());
        }

        // compute staking account signer seeds
        let staking_account_seeds = &[constants::STAKING_PDA_SEED.as_ref(), &[nonce_staking]];
        let staking_account_signer = &staking_account_seeds[..];

        // mint claimable amounts to user
        spl_token_mint(TokenMintParams {
            mint: ctx.accounts.nft_mint.to_account_info(),
            to: ctx.accounts.nft_to.to_account_info(),
            amount: amount,
            owner: ctx.accounts.staking_account.to_account_info(),
            owner_signer_seeds: staking_account_signer,
            token_program: ctx.accounts.token_program.to_account_info(),
        })?;

        Ok(())
    }
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = initializer,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
        // 8: account's signature on the anchor
        // 32: admin_key
        // 1: freeze_program
        // 32: authorized_creator
        // 4: authorized_name_starts Vec's length
        // 32 * 150: authorized_name_starts limit 150 and max_length 32
        // 4: active_rewards Vec's length
        // 32 * 150: active_rewards limit 150
        space = 8 + 32 + 1 + 32 + 4 + 32 * 150 + 4 + 32 * 150
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    #[account(mut)]
    pub initializer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct FreezeProgram<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct UpdateAdmin<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct UpdateAuthorizedCreator<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct AddAuthorizedNameStarts<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct RemoveAuthorizedNameStarts<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct AddReward<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(nonce_staking: u8)]
pub struct RemoveReward<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = nonce_staking,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    #[account(mut)]
    pub nft_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub nft_mint_authority_to: AccountInfo<'info>,

    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8, _nonce_user_staking: u8, _winner: Pubkey)]
pub struct AddWinner<'info> {
    pub nft_mint: UncheckedAccount<'info>,

    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    #[account(
        mut,
        seeds = [ _winner.as_ref() ],
        bump = _nonce_user_staking,
    )]
    pub user_staking_account: Box<Account<'info, UserStakingAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_nft_vault: u8, _nonce_staking: u8, _nonce_user_staking: u8)]
pub struct Stake<'info> {
    pub nft_mint: UncheckedAccount<'info>,

    pub nft_metadata: UncheckedAccount<'info>,

    #[account(mut)]
    pub nft_from: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub nft_from_authority: Signer<'info>,

    #[account(
        init,
        payer = nft_from_authority,
        token::mint = nft_mint,
        token::authority = staking_account,
        seeds = [ nft_mint.key().as_ref() ],
        bump = _nonce_nft_vault,
    )]
    pub nft_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
        constraint = !staking_account.freeze_program,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    #[account(
        init_if_needed,
        payer = nft_from_authority,
        seeds = [ nft_from_authority.key().as_ref() ],
        bump = _nonce_user_staking,
        // 8: account's signature on the anchor
        // 32: wallet
        // 4: nft_mint_keys Vec's length
        // 32 * 150: nft_mint_keys limit 150
        // 4: claimable Vec's length
        // (32 + 2) * 150: claimable limit 150
        space = 8 + 32 + 4 + 32 * 150 + 4 + (32 + 2) * 150,
    )]
    pub user_staking_account: Box<Account<'info, UserStakingAccount>>,

    ///used by anchor for init of the token
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(_nonce_nft_vault: u8, nonce_staking: u8, _nonce_user_staking: u8)]
pub struct Unstake<'info> {
    pub nft_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub nft_to: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub nft_to_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [ nft_mint.key().as_ref() ],
        bump = _nonce_nft_vault,
    )]
    pub nft_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = nonce_staking,
        constraint = !staking_account.freeze_program,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    #[account(
        mut,
        seeds = [ nft_to_authority.key().as_ref() ],
        bump = _nonce_user_staking,
    )]
    pub user_staking_account: Box<Account<'info, UserStakingAccount>>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(nonce_staking: u8, _nonce_user_staking: u8)]
pub struct Claim<'info> {
    #[account(mut)]
    pub nft_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub nft_to: Box<Account<'info, TokenAccount>>,

    pub nft_to_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = nonce_staking,
        constraint = !staking_account.freeze_program,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    #[account(
        mut,
        seeds = [ nft_to_authority.key().as_ref() ],
        bump = _nonce_user_staking,
    )]
    pub user_staking_account: Box<Account<'info, UserStakingAccount>>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(nonce_staking: u8)]
pub struct MintTo<'info> {
    #[account(mut)]
    pub nft_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub nft_to: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = nonce_staking,
        constraint = !staking_account.freeze_program,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(Default)]
pub struct StakingAccount {
    pub admin_key: Pubkey,
    pub freeze_program: bool,
    pub authorized_creator: Pubkey,
    pub authorized_name_starts: Vec<String>,
    pub active_rewards: Vec<Pubkey>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default)]
pub struct ClaimableToken {
    pub nft_mint: Pubkey,
    pub amount: u16,
}

#[account]
#[derive(Default)]
pub struct UserStakingAccount {
    pub wallet: Pubkey,
    pub nft_mint_keys: Vec<Pubkey>,
    pub claimable: Vec<ClaimableToken>,
}

#[error]
pub enum ErrorCode {
    #[msg("Not admin")]
    NotAdmin, // 6000, 0x1770
    #[msg("Invalid mint for reward")]
    InvalidMintForReward, // 6001, 0x1771
    #[msg("No authorized creators found in metadata")]
    NoAuthorizedCreatorsFoundInMetadata, // 6002, 0x1772
    #[msg("No authorized name start found in metadata")]
    NoAuthorizedNameStartFoundInMetadata, // 6003, 0x1773
    #[msg("Token transfer failed")]
    TokenTransferFailed, // 6004, 0x1774
    #[msg("Token mint failed")]
    TokenMintFailed, // 6005, 0x1775
    #[msg("Not staked item")]
    NotStakedItem, // 6006, 0x1776
    #[msg("Not claimable item")]
    NotClaimableItem, // 6007, 0x1777
    #[msg("Can't unstake before claim all rewards")]
    CantUnstakeBeforeClaim, // 6008, 0x1778
    #[msg("Close account failed")]
    CloseAccountFailed, // 6009, 0x1779
}

// Asserts the signer is admin
fn is_admin<'info>(
    staking_account: &Account<'info, StakingAccount>,
    signer: &Signer<'info>,
) -> Result<()> {
    if staking_account.admin_key != *signer.key {
        return Err(ErrorCode::NotAdmin.into());
    }

    Ok(())
}
