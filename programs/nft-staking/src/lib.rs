pub mod utils;

use crate::utils::{spl_token_transfer, TokenTransferParams};
use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use metaplex_token_metadata::state::Metadata;
use spl_token::instruction::AuthorityType;
use std::convert::TryInto;

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
        _nonce_token_vault: u8,
        _nonce_staking: u8,
        authorized_creator: Pubkey,
        rewards_per_ts: u64,
    ) -> ProgramResult {
        ctx.accounts.staking_account.admin_key = *ctx.accounts.initializer.key;
        ctx.accounts.staking_account.authorized_creator = authorized_creator;
        ctx.accounts.staking_account.rewards_per_ts = rewards_per_ts;

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
    pub fn add_reward(
        ctx: Context<AddReward>,
        _nonce_staking: u8,
        nft_mint: Pubkey,
    ) -> ProgramResult {
        for reward in &ctx.accounts.staking_account.active_rewards {
            if reward == &nft_mint {
                return Err(ErrorCode::InvalidMintForReward.into());
            }
        }

        // add active reward
        ctx.accounts.staking_account.active_rewards.push(nft_mint);

        Ok(())
    }

    pub fn remove_reward(ctx: Context<RemoveReward>, nonce_staking: u8) -> ProgramResult {
        for i in 0..ctx.accounts.staking_account.active_rewards.len() {
            if ctx.accounts.staking_account.active_rewards[i] == *ctx.accounts.nft_mint.key {
                // remove active reward
                ctx.accounts.staking_account.active_rewards.remove(i);

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
        }
        return Err(ErrorCode::InvalidMintForReward.into());
    }

    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn update_rewards_per_ts(
        ctx: Context<UpdateRewardsPerTs>,
        _nonce_staking: u8,
        new_rewards_per_ts: u64,
    ) -> ProgramResult {
        ctx.accounts.staking_account.rewards_per_ts = new_rewards_per_ts;

        Ok(())
    }

    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn toggle_freeze_program(ctx: Context<FreezeProgram>, _nonce_staking: u8) -> ProgramResult {
        ctx.accounts.staking_account.freeze_program = !ctx.accounts.staking_account.freeze_program;

        Ok(())
    }

    pub fn stake(
        ctx: Context<Stake>,
        nonce_token_vault: u8,
        _nonce_nft_vault: u8,
        _nonce_staking: u8,
        _nonce_user_staking: u8,
    ) -> ProgramResult {
        let nft_metadata = &ctx.accounts.nft_metadata;
        let metadata = Metadata::from_account_info(&nft_metadata)?;

        match metadata.data.creators {
            Some(creators) => {
                for creator in creators {
                    if creator.verified
                        && creator.address == ctx.accounts.staking_account.authorized_creator
                    {
                        // transfer nft to nft vault
                        spl_token_transfer(TokenTransferParams {
                            source: ctx.accounts.nft_from.to_account_info(),
                            destination: ctx.accounts.nft_vault.to_account_info(),
                            authority: ctx.accounts.nft_from_authority.to_account_info(),
                            authority_signer_seeds: &[],
                            token_program: ctx.accounts.token_program.to_account_info(),
                            amount: 1,
                        })?;

                        // transfer rewards to user
                        // compute token vault signer seeds
                        // let token_mint_key = ctx.accounts.token_mint.key();
                        // let seeds = &[token_mint_key.as_ref(), &[nonce_token_vault]];
                        // let signer = &seeds[..];
                        // let rewards = update_user_staking(
                        //     &ctx.accounts.staking_account,
                        //     &mut ctx.accounts.user_staking_account,
                        // );

                        // spl_token_transfer(TokenTransferParams {
                        //     source: ctx.accounts.token_vault.to_account_info(),
                        //     destination: ctx.accounts.token_to.to_account_info(),
                        //     authority: ctx.accounts.token_vault.to_account_info(),
                        //     authority_signer_seeds: signer,
                        //     token_program: ctx.accounts.token_program.to_account_info(),
                        //     amount: rewards,
                        // })?;

                        // update user staking hashes & nft_mint_keys
                        ctx.accounts.user_staking_account.hashes =
                            (ctx.accounts.user_staking_account.hashes as u128)
                                .checked_add(1)
                                .unwrap()
                                .try_into()
                                .unwrap();

                        // push nft_mint_key from the nft_mint_keys
                        ctx.accounts
                            .user_staking_account
                            .nft_mint_keys
                            .push(*ctx.accounts.nft_mint.key);

                        // update staking total hashes
                        ctx.accounts.staking_account.total_hashes =
                            (ctx.accounts.staking_account.total_hashes as u128)
                                .checked_add(1)
                                .unwrap()
                                .try_into()
                                .unwrap();

                        return Ok(());
                    }
                }

                return Err(ErrorCode::NoCreatorsFoundInMetadata.into());
            }
            None => {
                return Err(ErrorCode::NoCreatorsFoundInMetadata.into());
            }
        };
    }

    pub fn unstake(
        ctx: Context<Unstake>,
        nonce_token_vault: u8,
        nonce_nft_vault: u8,
        _nonce_staking: u8,
        _nonce_user_staking: u8,
    ) -> ProgramResult {
        let nft_mint_keys = &ctx.accounts.user_staking_account.nft_mint_keys;

        if nft_mint_keys.is_empty() {
            return Err(ErrorCode::NotStakedItem.into());
        } else {
            for i in 0..nft_mint_keys.len() {
                if nft_mint_keys[i] == *ctx.accounts.nft_mint.key {
                    // transfer nft to user
                    // compute nft vault signer seeds
                    let nft_mint_key = ctx.accounts.nft_mint.key();
                    let nft_seeds = &[nft_mint_key.as_ref(), &[nonce_nft_vault]];
                    let nft_signer = &nft_seeds[..];

                    spl_token_transfer(TokenTransferParams {
                        source: ctx.accounts.nft_vault.to_account_info(),
                        destination: ctx.accounts.nft_to.to_account_info(),
                        authority: ctx.accounts.nft_vault.to_account_info(),
                        authority_signer_seeds: nft_signer,
                        token_program: ctx.accounts.token_program.to_account_info(),
                        amount: 1,
                    })?;

                    // transfer rewards to user
                    // compute token vault signer seeds
                    // let token_mint_key = ctx.accounts.token_mint.key();
                    // let token_seeds = &[token_mint_key.as_ref(), &[nonce_token_vault]];
                    // let token_signer = &token_seeds[..];
                    // let rewards = update_user_staking(
                    //     &ctx.accounts.staking_account,
                    //     &mut ctx.accounts.user_staking_account,
                    // );

                    // spl_token_transfer(TokenTransferParams {
                    //     source: ctx.accounts.token_vault.to_account_info(),
                    //     destination: ctx.accounts.token_to.to_account_info(),
                    //     authority: ctx.accounts.token_vault.to_account_info(),
                    //     authority_signer_seeds: token_signer,
                    //     token_program: ctx.accounts.token_program.to_account_info(),
                    //     amount: rewards,
                    // })?;

                    // update user staking hashes & nft_mint_keys
                    ctx.accounts.user_staking_account.hashes =
                        (ctx.accounts.user_staking_account.hashes as u128)
                            .checked_sub(1)
                            .unwrap()
                            .try_into()
                            .unwrap();

                    // pop nft_mint_key from the nft_mint_keys
                    ctx.accounts.user_staking_account.nft_mint_keys.remove(i);

                    // update staking total hashes
                    ctx.accounts.staking_account.total_hashes =
                        (ctx.accounts.staking_account.total_hashes as u128)
                            .checked_sub(1)
                            .unwrap()
                            .try_into()
                            .unwrap();

                    return Ok(());
                }
            }
        }

        return Err(ErrorCode::NotStakedItem.into());
    }

    pub fn claim(
        ctx: Context<Claim>,
        nonce_token_vault: u8,
        _nonce_staking: u8,
        _nonce_user_staking: u8,
    ) -> ProgramResult {
        // transfer rewards to user
        // compute token vault signer seeds
        // let token_mint_key = ctx.accounts.token_mint.key();
        // let token_seeds = &[token_mint_key.as_ref(), &[nonce_token_vault]];
        // let token_signer = &token_seeds[..];
        // let rewards = update_user_staking(
        //     &ctx.accounts.staking_account,
        //     &mut ctx.accounts.user_staking_account,
        // );

        // spl_token_transfer(TokenTransferParams {
        //     source: ctx.accounts.token_vault.to_account_info(),
        //     destination: ctx.accounts.token_to.to_account_info(),
        //     authority: ctx.accounts.token_vault.to_account_info(),
        //     authority_signer_seeds: token_signer,
        //     token_program: ctx.accounts.token_program.to_account_info(),
        //     amount: rewards,
        // })?;

        return Ok(());
    }
}

pub fn update_user_staking<'info>(
    staking_account: &ProgramAccount<'info, StakingAccount>,
    user_staking_account: &mut ProgramAccount<'info, UserStakingAccount>,
) -> u64 {
    //user staked seconds
    let now_ts = Clock::get().unwrap().unix_timestamp;
    let ts_passed: u64 = (now_ts as u128)
        .checked_sub(user_staking_account.last_updated_ts as u128)
        .unwrap()
        .try_into()
        .unwrap();
    user_staking_account.last_updated_ts = now_ts as u64;

    //rewards = rewards_per_ts * ts_passed * hashes
    if user_staking_account.hashes == 0 {
        return 0;
    } else {
        let reward: u64 = (staking_account.rewards_per_ts as u128)
            .checked_mul(ts_passed as u128)
            .unwrap()
            .checked_mul(user_staking_account.hashes as u128)
            .unwrap()
            .try_into()
            .unwrap();

        return reward;
    }
}

#[derive(Accounts)]
#[instruction(_nonce_token_vault: u8, _nonce_staking: u8)]
pub struct Initialize<'info> {
    #[account(
        address = constants::DAO_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub token_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = initializer,
        token::mint = token_mint,
        token::authority = token_vault,
        seeds = [ constants::DAO_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap().as_ref() ],
        bump = _nonce_token_vault,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = initializer,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: ProgramAccount<'info, StakingAccount>,

    #[account(mut)]
    pub initializer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct UpdateAdmin<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: ProgramAccount<'info, StakingAccount>,

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
    pub staking_account: ProgramAccount<'info, StakingAccount>,

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
    pub staking_account: ProgramAccount<'info, StakingAccount>,

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
    pub staking_account: ProgramAccount<'info, StakingAccount>,

    #[account(mut)]
    pub nft_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub nft_mint_authority_to: AccountInfo<'info>,

    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct UpdateRewardsPerTs<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: ProgramAccount<'info, StakingAccount>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct FreezeProgram<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: ProgramAccount<'info, StakingAccount>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(nonce_token_vault: u8, _nonce_nft_vault: u8, _nonce_staking: u8, _nonce_user_staking: u8)]
pub struct Stake<'info> {
    #[account(
        address = constants::DAO_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub token_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub token_to: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [ token_mint.key().as_ref() ],
        bump = nonce_token_vault,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
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
        token::authority = nft_vault,
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
    pub staking_account: ProgramAccount<'info, StakingAccount>,

    #[account(
        init_if_needed,
        payer = nft_from_authority,
        seeds = [ nft_from_authority.key().as_ref() ],
        bump = _nonce_user_staking,
    )]
    pub user_staking_account: ProgramAccount<'info, UserStakingAccount>,

    ///used by anchor for init of the token
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(nonce_token_vault: u8, nonce_nft_vault: u8, _nonce_staking: u8, _nonce_user_staking: u8)]
pub struct Unstake<'info> {
    #[account(
        address = constants::DAO_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub token_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub token_to: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [ token_mint.key().as_ref() ],
        bump = nonce_token_vault,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub nft_mint: UncheckedAccount<'info>,

    #[account(mut)]
    pub nft_to: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub nft_to_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [ nft_mint.key().as_ref() ],
        bump = nonce_nft_vault,
        close = nft_to_authority,
    )]
    pub nft_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
        constraint = !staking_account.freeze_program,
    )]
    pub staking_account: ProgramAccount<'info, StakingAccount>,

    #[account(
        mut,
        seeds = [ nft_to_authority.key().as_ref() ],
        bump = _nonce_user_staking,
    )]
    pub user_staking_account: ProgramAccount<'info, UserStakingAccount>,

    ///used by anchor for init of the token
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(nonce_token_vault: u8, _nonce_staking: u8, _nonce_user_staking: u8)]
pub struct Claim<'info> {
    #[account(
        address = constants::DAO_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub token_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub token_to: Box<Account<'info, TokenAccount>>,

    pub token_to_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [ token_mint.key().as_ref() ],
        bump = nonce_token_vault,
    )]
    pub token_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
        constraint = !staking_account.freeze_program,
    )]
    pub staking_account: ProgramAccount<'info, StakingAccount>,

    #[account(
        mut,
        seeds = [ token_to_authority.key().as_ref() ],
        bump = _nonce_user_staking,
    )]
    pub user_staking_account: ProgramAccount<'info, UserStakingAccount>,

    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(Default)]
pub struct StakingAccount {
    pub admin_key: Pubkey,
    pub active_rewards: Vec<Pubkey>,
    pub authorized_creator: Pubkey,
    pub freeze_program: bool,
    pub rewards_per_ts: u64,
    pub total_hashes: u64,
}

#[account]
#[derive(Default)]
pub struct UserStakingAccount {
    pub last_updated_ts: u64,
    pub hashes: u64,
    pub nft_mint_keys: Vec<Pubkey>,
}

#[error]
pub enum ErrorCode {
    #[msg("Not admin")]
    NotAdmin,
    #[msg("Invalid mint for reward")]
    InvalidMintForReward,
    #[msg("No creators found in metadata")]
    NoCreatorsFoundInMetadata,
    #[msg("Token transfer failed")]
    TokenTransferFailed,
    #[msg("Not staked item")]
    NotStakedItem,
}

// Asserts the signer is admin
fn is_admin<'info>(
    staking_account: &ProgramAccount<'info, StakingAccount>,
    signer: &Signer<'info>,
) -> Result<()> {
    if staking_account.admin_key != *signer.key {
        return Err(ErrorCode::NotAdmin.into());
    }

    Ok(())
}
