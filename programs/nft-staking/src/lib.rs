pub mod utils;

use crate::utils::*;
use anchor_lang::{prelude::*, AccountsClose};
use anchor_spl::token::{Mint, Token, TokenAccount};
use spl_token::{instruction::AuthorityType, state::AccountState};

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

#[cfg(not(feature = "local-testing"))]
pub mod constants {
    pub const AURY_TOKEN_MINT_PUBKEY: &str = "AURYydfxJib1ZkTir1Jn1J9ECYUtjb6rKQVmtYaixWPP";
    pub const STAKING_PDA_SEED: &[u8] = b"nft_staking";
}

#[cfg(feature = "local-testing")]
pub mod constants {
    pub const AURY_TOKEN_MINT_PUBKEY: &str = "teST1ieLrLdr4MJPZ7i8mgSCLQ7rTrPRjNnyFdHFaz9";
    pub const STAKING_PDA_SEED: &[u8] = b"nft_staking";
}

#[program]
pub mod nft_staking {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        _nonce_staking: u8,
        _nonce_aury_vault: u8,
        authorized_creator: Pubkey,
        authorized_name_starts: Vec<String>,
        minimum_staking_period: u64,
        maximum_staking_period: u64,
    ) -> ProgramResult {
        if !(minimum_staking_period < maximum_staking_period && minimum_staking_period > 0) {
            return Err(ErrorCode::InvalidStakingPeriod.into());
        }

        ctx.accounts.staking_account.admin_key = *ctx.accounts.initializer.key;
        ctx.accounts.staking_account.authorized_creator = authorized_creator;
        ctx.accounts
            .staking_account
            .authorized_name_starts
            .extend(authorized_name_starts);
        ctx.accounts.staking_account.minimum_staking_period = minimum_staking_period;
        ctx.accounts.staking_account.maximum_staking_period = maximum_staking_period;

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
    pub fn update_staking_period(
        ctx: Context<UpdateStakingPeriod>,
        _nonce_staking: u8,
        new_minimum_staking_period: u64,
        new_maximum_staking_period: u64,
    ) -> ProgramResult {
        if !(new_minimum_staking_period < new_maximum_staking_period
            && new_minimum_staking_period > 0)
        {
            return Err(ErrorCode::InvalidStakingPeriod.into());
        }

        ctx.accounts.staking_account.minimum_staking_period = new_minimum_staking_period;
        ctx.accounts.staking_account.maximum_staking_period = new_maximum_staking_period;

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

    // maximum size is 10
    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn remove_reward<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, RemoveReward<'info>>, 
        nonce_staking: u8
    ) -> ProgramResult {
        // determine the remaining accounts
        let remaining_accounts = ctx.remaining_accounts;
        let remaining_accounts_length = ctx.remaining_accounts.len();

        if remaining_accounts_length == 0 {
            return Err(ErrorCode::InvalidAccounts.into());
        }

        let mut index = 0;
        while index < remaining_accounts_length {
            let nft_mint = &remaining_accounts[index];

            match ctx
                .accounts
                .staking_account
                .active_rewards
                .iter()
                .position(|active_reward| active_reward == nft_mint.key)
            {
                Some(index) => {
                    // remove active reward
                    ctx.accounts.staking_account.active_rewards.remove(index);

                    // compute staking account signer seeds
                    let staking_account_seeds =
                        &[constants::STAKING_PDA_SEED.as_ref(), &[nonce_staking]];
                    let staking_account_signer = &staking_account_seeds[..];

                    // transfer nft mint authority
                    spl_set_authority(SetAuthorityParams {
                        account: nft_mint.clone(),
                        new_authority: ctx.accounts.nft_mint_authority_to.to_account_info(),
                        authority_type: AuthorityType::MintTokens,
                        owner: ctx.accounts.staking_account.to_account_info(),
                        owner_signer_seeds: staking_account_signer,
                        token_program: ctx.accounts.token_program.to_account_info(),
                    })?;
                }
                None => {
                    return Err(ErrorCode::InvalidMintForReward.into());
                }
            }

            index += 1;
        }

        Ok(())
    }

    // maximum size is 15
    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn add_winner<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, AddWinner<'info>>,
        _nonce_staking: u8,
        winner_staking_index: Vec<u32>,
        winner: Vec<Pubkey>
    ) -> ProgramResult {
        // determine the remaining accounts
        let remaining_accounts = ctx.remaining_accounts;
        let remaining_accounts_length = ctx.remaining_accounts.len();

        if remaining_accounts_length % 2 != 0 || remaining_accounts_length / 2 != winner_staking_index.len() || remaining_accounts_length / 2 != winner.len() {
            return Err(ErrorCode::InvalidAccounts.into());
        }

        let mut index = 0;
        while index < remaining_accounts_length {
            let nft_mint = &remaining_accounts[index];
            let mut user_staking_account = Account::<'_, UserStakingAccount>::try_from(&remaining_accounts[index + 1])?;

            // determine if stake is locked
            if user_staking_account.staking_period == 0 {
                return Err(ErrorCode::StakingNotLocked.into());
            }

            // determine user_staking_account pda
            if user_staking_account.to_account_info().owner != &id() || user_staking_account.index != winner_staking_index[index / 2] || user_staking_account.wallet != winner[index / 2] {
                return Err(ErrorCode::InvalidAccounts.into());
            }

            // Check if nft is one of the rewards
            if ctx
                .accounts
                .staking_account
                .active_rewards
                .iter()
                .find(|&active_reward| active_reward == nft_mint.key)
                == None
            {
                return Err(ErrorCode::InvalidMintForReward.into());
            }

            match user_staking_account
                .claimable
                .iter()
                .position(|claimable_token| claimable_token.nft_mint == *nft_mint.key)
            {
                Some(index) => {
                    user_staking_account.claimable[index].amount += 1;
                }
                None => {
                    user_staking_account
                        .claimable
                        .push(ClaimableToken {
                            nft_mint: *nft_mint.key,
                            amount: 1,
                        });
                }
            }
            user_staking_account.exit(&id())?;

            index += 2;
        }

        Ok(())
    }

    // maximum size is 10
    #[access_control(is_admin(&ctx.accounts.staking_account, &ctx.accounts.admin))]
    pub fn add_aury_winner<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, AddAuryWinner<'info>>,
        _nonce_staking: u8,
        _nonce_aury_vault: u8,
        winner_staking_index: Vec<u32>,
        winner: Vec<Pubkey>,
        aury_amount: Vec<u64>,
    ) -> ProgramResult {
        // determine the remaining accounts
        let remaining_accounts = ctx.remaining_accounts;
        let remaining_accounts_length = ctx.remaining_accounts.len();

        if remaining_accounts_length == 0 || remaining_accounts_length != winner_staking_index.len() || remaining_accounts_length != winner.len() || remaining_accounts_length != aury_amount.len() {
            return Err(ErrorCode::InvalidAccounts.into());
        }

        let mut index = 0;
        while index < remaining_accounts_length {
            let mut user_staking_account = Account::<'_, UserStakingAccount>::try_from(&remaining_accounts[index])?;

            // determine if stake is locked
            if user_staking_account.staking_period == 0 {
                return Err(ErrorCode::StakingNotLocked.into());
            }

            // determine user_staking_account pda
            if user_staking_account.to_account_info().owner != &id() || user_staking_account.index != winner_staking_index[index / 2] || user_staking_account.wallet != winner[index / 2] {
                return Err(ErrorCode::InvalidAccounts.into());
            }

            // transfer aury to the vault
            spl_token_transfer(TokenTransferParams {
                source: ctx.accounts.aury_from.to_account_info(),
                destination: ctx.accounts.aury_vault.to_account_info(),
                amount: aury_amount[index],
                authority: ctx.accounts.admin.to_account_info(),
                authority_signer_seeds: &[],
                token_program: ctx.accounts.token_program.to_account_info(),
            })?;

            // update user staking info
            user_staking_account.claimable_aury_amount += aury_amount[index];
            user_staking_account.exit(&id())?;

            index += 1;
        }

        Ok(())
    }

    // maximum size is 4
    pub fn stake<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, Stake<'info>>,
        nonce_nft_vault: Vec<u8>,
        _nonce_staking: u8,
        _nonce_user_staking_counter: u8,
        _nonce_user_staking: u8,
    ) -> ProgramResult {
        // determine if stake is locked
        if ctx.accounts.user_staking_account.staking_period > 0 {
            return Err(ErrorCode::StakingLocked.into());
        }

        // determine the remaining accounts
        let remaining_accounts = ctx.remaining_accounts;
        let remaining_accounts_length = ctx.remaining_accounts.len();

        if remaining_accounts_length % 4 != 0
        || nonce_nft_vault.len() != remaining_accounts_length / 4 {
            return Err(ErrorCode::InvalidAccounts.into());
        }

        let nft_from_authority = &ctx.accounts.nft_from_authority;
        let owner = &ctx.accounts.staking_account;
        let system_program = &ctx.accounts.system_program;
        let token_program = &ctx.accounts.token_program;
        let rent = &ctx.accounts.rent;

        let mut index = 0;
        while index < remaining_accounts_length {
            let nft_mint = &remaining_accounts[index];
            let nft_metadata = &remaining_accounts[index + 1];
            let nft_from = Account::<'_, TokenAccount>::try_from(&remaining_accounts[index + 2])?;
            let nft_vault = &remaining_accounts[index + 3];

            assert_metadata_valid(
                nft_metadata,
                nft_mint.key,
                ctx.accounts.staking_account.clone(),
            )?;

            // init if needed nft vault
            if nft_vault.owner == &token_program.key() {
                let nft_vault_token_account = Account::<'_, TokenAccount>::try_from(&nft_vault)?;

                // validate the existing nft vault
                if nft_vault_token_account.mint != *nft_mint.key
                    || nft_vault_token_account.owner != owner.key()
                    || nft_vault_token_account.state != AccountState::Initialized
                {
                    return Err(ErrorCode::InvalidAccounts.into());
                }
            } else {
                // compute nft vault account signer seeds
                let nft_vault_account_seeds = &[
                    nft_from_authority.key.as_ref(),
                    nft_mint.key.as_ref(),
                    &[nonce_nft_vault[index / 4]],
                ];
                let nft_vault_account_signer = &nft_vault_account_seeds[..];

                // initialize nft vault account
                spl_init_token_account(InitializeTokenAccountParams {
                    account: nft_vault.clone(),
                    account_signer_seeds: nft_vault_account_signer,
                    mint: nft_mint.clone(),
                    owner: owner.to_account_info(),
                    payer: nft_from_authority.to_account_info(),
                    system_program: system_program.to_account_info(),
                    token_program: token_program.to_account_info(),
                    rent: rent.to_account_info(),
                })?;
            }

            // transfer nft to nft vault
            spl_token_transfer(TokenTransferParams {
                source: nft_from.to_account_info(),
                destination: nft_vault.clone(),
                authority: nft_from_authority.to_account_info(),
                authority_signer_seeds: &[],
                token_program: token_program.to_account_info(),
                amount: 1,
            })?;

            // push nft_mint_key from the nft_mint_keys
            ctx.accounts
                .user_staking_account
                .nft_mint_keys
                .push(*nft_mint.key);

            index += 4;
        }

        Ok(())
    }

    pub fn lock_stake(
        ctx: Context<LockStake>,
        _nonce_staking: u8,
        _nonce_user_staking_counter: u8,
        _nonce_user_staking: u8,
        _nonce_aury_vault: u8,
        staking_period: u64,
        aury_amount: u64,
    ) -> ProgramResult {
        // determine if stake is locked
        if ctx.accounts.user_staking_account.staking_period > 0 {
            return Err(ErrorCode::StakingLocked.into());
        }

        // determine the staking period
        if !(staking_period >= ctx.accounts.staking_account.minimum_staking_period
            && staking_period <= ctx.accounts.staking_account.maximum_staking_period)
        {
            return Err(ErrorCode::InvalidStakingPeriod.into());
        }

        // set user staking info
        ctx.accounts.user_staking_account.index = ctx.accounts.user_staking_counter_account.counter;
        ctx.accounts.user_staking_account.wallet = *ctx.accounts.nft_from_authority.key;
        ctx.accounts.user_staking_account.staking_at = Clock::get()?.unix_timestamp as u64;
        ctx.accounts.user_staking_account.staking_period = staking_period;
        ctx.accounts.user_staking_counter_account.counter += 1;

        if aury_amount != 0 {
            // transfer aury to the vault
            spl_token_transfer(TokenTransferParams {
                source: ctx.accounts.aury_from.to_account_info(),
                destination: ctx.accounts.aury_vault.to_account_info(),
                amount: aury_amount,
                authority: ctx.accounts.nft_from_authority.to_account_info(),
                authority_signer_seeds: &[],
                token_program: ctx.accounts.token_program.to_account_info(),
            })?;

            // update user staking info
            ctx.accounts.user_staking_account.aury_deposit = aury_amount;
        }
        

        Ok(())
    }

    // maximum size is 5
    pub fn unstake<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, Unstake<'info>>,
        nonce_staking: u8,
        _user_staking_index: u32,
        _nonce_user_staking: u8,
    ) -> ProgramResult {
        // determine if claimable is empty
        if ctx.accounts.user_staking_account.claimable.len() > 0
            || ctx.accounts.user_staking_account.claimable_aury_amount > 0
        {
            return Err(ErrorCode::CantUnstakeBeforeClaim.into());
        }

        // determine if stake is locked
        if ctx.accounts.user_staking_account.staking_period == 0 {
            return Err(ErrorCode::StakingNotLocked.into());
        }

        // determine the staking period
        if (Clock::get()?.unix_timestamp as u64 - ctx.accounts.user_staking_account.staking_at)
            < ctx.accounts.user_staking_account.staking_period
        {
            return Err(ErrorCode::StakingLocked.into());
        }

        // determine the remaining accounts
        let remaining_accounts = ctx.remaining_accounts;
        let remaining_accounts_length = ctx.remaining_accounts.len();

        if remaining_accounts_length % 2 != 0
            || remaining_accounts_length / 2 > ctx.accounts.user_staking_account.nft_mint_keys.len()
        {
            return Err(ErrorCode::InvalidAccounts.into());
        }

        let authority = &ctx.accounts.staking_account;
        let nft_to_authority = &ctx.accounts.nft_to_authority;
        let token_program = &ctx.accounts.token_program;
        // compute staking account signer seeds
        let staking_account_seeds = &[constants::STAKING_PDA_SEED.as_ref(), &[nonce_staking]];
        let staking_account_signer = &staking_account_seeds[..];

        let mut index = 0;
        while index < remaining_accounts_length {
            let nft_to = Account::<'_, TokenAccount>::try_from(&remaining_accounts[index])?;
            let mut nft_vault = Account::<'_, TokenAccount>::try_from(&remaining_accounts[index + 1])?;

            match ctx
                .accounts
                .user_staking_account
                .nft_mint_keys
                .iter()
                .position(|&mint_key| mint_key == nft_vault.mint)
            {
                Some(index) => {
                    // remove staked nft key
                    ctx.accounts
                        .user_staking_account
                        .nft_mint_keys
                        .remove(index);

                    // transfer nft to user
                    spl_token_transfer(TokenTransferParams {
                        source: nft_vault.to_account_info(),
                        destination: nft_to.to_account_info(),
                        authority: authority.to_account_info(),
                        authority_signer_seeds: staking_account_signer,
                        token_program: token_program.to_account_info(),
                        amount: 1,
                    })?;

                    // Close nft_vault tokenAccount
                    (&mut nft_vault).reload()?;

                    if nft_vault.amount == 0 {
                        spl_close_account(CloseAccountParams {
                            account: nft_vault.to_account_info(),
                            destination: nft_to_authority.to_account_info(),
                            owner: authority.to_account_info(),
                            owner_signer_seeds: staking_account_signer,
                            token_program: token_program.to_account_info(),
                        })?;
                    }
                }
                None => {
                    return Err(ErrorCode::NotStakedItem.into());
                }
            }

            index += 2;
        }

        // close account if it's empty
        if ctx.accounts.user_staking_account.nft_mint_keys.len() == 0 {
            ctx.accounts.user_staking_account.close(ctx.accounts.nft_to_authority.to_account_info())?;
        }

        Ok(())
    }

    // maximum size is 5
    pub fn claim<'a, 'b, 'c, 'info>(
        ctx: Context<'a, 'b, 'c, 'info, Claim<'info>>,
        nonce_staking: u8,
        _user_staking_index: u32,
        _nonce_user_staking: u8,
    ) -> ProgramResult {
        // determine the remaining accounts
        let remaining_accounts = ctx.remaining_accounts;
        let remaining_accounts_length = ctx.remaining_accounts.len();

        if remaining_accounts_length == 0
        || remaining_accounts_length % 2 != 0 {
            return Err(ErrorCode::InvalidAccounts.into());
        }

        let mut index = 0;
        while index < remaining_accounts_length {
            let nft_mint = &remaining_accounts[index];
            let nft_to = Account::<'_, TokenAccount>::try_from(&remaining_accounts[index + 1])?;

            match ctx
                .accounts
                .user_staking_account
                .claimable
                .iter()
                .position(|claimable_token| claimable_token.nft_mint == *nft_mint.key)
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
                        continue;
                    }

                    // compute staking account signer seeds
                    let staking_account_seeds =
                        &[constants::STAKING_PDA_SEED.as_ref(), &[nonce_staking]];
                    let staking_account_signer = &staking_account_seeds[..];

                    // mint claimable amounts to user
                    spl_token_mint(TokenMintParams {
                        mint: nft_mint.clone(),
                        to: nft_to.to_account_info(),
                        amount: claimable_token.amount as u64,
                        owner: ctx.accounts.staking_account.to_account_info(),
                        owner_signer_seeds: staking_account_signer,
                        token_program: ctx.accounts.token_program.to_account_info(),
                    })?;
                }
                None => {
                    return Err(ErrorCode::NotClaimableItem.into());
                }
            }

            index += 2;
        }

        Ok(())
    }

    pub fn claim_aury_reward(
        ctx: Context<ClaimAuryReward>,
        nonce_aury_vault: u8,
        _user_staking_index: u32,
        _nonce_user_staking: u8,
    ) -> ProgramResult {
        if ctx.accounts.user_staking_account.claimable_aury_amount > 0 {
            // compute aury vault account signer seeds
            let aury_mint_key = ctx.accounts.aury_mint.key();
            let aury_vault_account_seeds = &[aury_mint_key.as_ref(), &[nonce_aury_vault]];
            let aury_vault_account_signer = &aury_vault_account_seeds[..];

            // transfer aury from vault
            spl_token_transfer(TokenTransferParams {
                source: ctx.accounts.aury_vault.to_account_info(),
                destination: ctx.accounts.aury_to.to_account_info(),
                amount: ctx.accounts.user_staking_account.claimable_aury_amount,
                authority: ctx.accounts.aury_vault.to_account_info(),
                authority_signer_seeds: aury_vault_account_signer,
                token_program: ctx.accounts.token_program.to_account_info(),
            })?;

            ctx.accounts.user_staking_account.claimable_aury_amount = 0;
        }

        Ok(())
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
#[instruction(_nonce_staking: u8, _nonce_aury_vault: u8)]
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

    #[account(
        address = constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub aury_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = initializer,
        token::mint = aury_mint,
        token::authority = aury_vault,
        seeds = [ constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap().as_ref() ],
        bump = _nonce_aury_vault,
    )]
    pub aury_vault: Box<Account<'info, TokenAccount>>,

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
pub struct UpdateStakingPeriod<'info> {
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
    pub nft_mint_authority_to: AccountInfo<'info>,

    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8)]
pub struct AddWinner<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    pub admin: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8, _nonce_aury_vault: u8)]
pub struct AddAuryWinner<'info> {
    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    #[account(
        address = constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub aury_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [ aury_mint.key().as_ref() ],
        bump = _nonce_aury_vault,
    )]
    pub aury_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub aury_from: Box<Account<'info, TokenAccount>>,

    pub admin: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction[nonce_nft_vault: Vec<u8>, _nonce_staking: u8, _nonce_user_staking_counter: u8, _nonce_user_staking: u8]]
pub struct Stake<'info> {
    pub nft_from_authority: Signer<'info>,

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
        bump = _nonce_user_staking_counter,
    )]
    pub user_staking_counter_account: Box<Account<'info, UserStakingCounterAccount>>,

    #[account(
        init_if_needed,
        payer = nft_from_authority,
        seeds = [ user_staking_counter_account.counter.to_string().as_ref(), nft_from_authority.key().as_ref() ],
        bump = _nonce_user_staking,
        // 8: account's signature on the anchor
        // 4: index
        // 32: wallet
        // 4: nft_mint_keys Vec's length
        // 32 * 10: nft_mint_keys limit 10
        // 4: claimable Vec's length
        // (32 + 2) * 5: claimable limit 5
        // 8: staking_at
        // 8: staking_period
        // 8: claimable aury amount
        // 8: aury_deposit
        space = 8 + 4 + 32 + 4 + 32 * 10 + 4 + (32 + 2) * 5 + 8 + 8 + 8 + 8, 
    )]
    pub user_staking_account: Box<Account<'info, UserStakingAccount>>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(_nonce_staking: u8, _nonce_user_staking_counter: u8, _nonce_user_staking: u8, _nonce_aury_vault: u8)]
pub struct LockStake<'info> {
    #[account(mut)]
    pub nft_from_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [ constants::STAKING_PDA_SEED.as_ref() ],
        bump = _nonce_staking,
        constraint = !staking_account.freeze_program,
    )]
    pub staking_account: Box<Account<'info, StakingAccount>>,

    #[account(
        mut,
        seeds = [ nft_from_authority.key().as_ref() ],
        bump = _nonce_user_staking_counter,
    )]
    pub user_staking_counter_account: Box<Account<'info, UserStakingCounterAccount>>,

    #[account(
        mut,
        seeds = [ user_staking_counter_account.counter.to_string().as_ref(), nft_from_authority.key().as_ref() ],
        bump = _nonce_user_staking,
    )]
    pub user_staking_account: Box<Account<'info, UserStakingAccount>>,
    #[account(
        address = constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub aury_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [ aury_mint.key().as_ref() ],
        bump = _nonce_aury_vault,
    )]
    pub aury_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub aury_from: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(nonce_staking: u8, _user_staking_index: u32, _nonce_user_staking: u8)]
pub struct Unstake<'info> {
    #[account(mut)]
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
        seeds = [ _user_staking_index.to_string().as_ref(), nft_to_authority.key().as_ref() ],
        bump = _nonce_user_staking,
    )]
    pub user_staking_account: Box<Account<'info, UserStakingAccount>>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(nonce_staking: u8, _user_staking_index: u32, _nonce_user_staking: u8)]
pub struct Claim<'info> {
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
        seeds = [ _user_staking_index.to_string().as_ref(), nft_to_authority.key().as_ref() ],
        bump = _nonce_user_staking,
    )]
    pub user_staking_account: Box<Account<'info, UserStakingAccount>>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(nonce_aury_vault: u8, _user_staking_index: u32, _nonce_user_staking: u8)]
pub struct ClaimAuryReward<'info> {
    #[account(
        address = constants::AURY_TOKEN_MINT_PUBKEY.parse::<Pubkey>().unwrap(),
    )]
    pub aury_mint: Box<Account<'info, Mint>>,

    #[account(
        mut,
        seeds = [ aury_mint.key().as_ref() ],
        bump = nonce_aury_vault,
    )]
    pub aury_vault: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub aury_to: Box<Account<'info, TokenAccount>>,

    pub aury_to_authority: Signer<'info>,

    #[account(
        mut,
        seeds = [ _user_staking_index.to_string().as_ref(), aury_to_authority.key().as_ref() ],
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
    pub minimum_staking_period: u64,
    pub maximum_staking_period: u64,
    pub active_rewards: Vec<Pubkey>,
}

#[account]
#[derive(Default)]
pub struct UserStakingCounterAccount {
    pub counter: u32,
}

#[derive(AnchorSerialize, AnchorDeserialize, Copy, Clone, Default)]
pub struct ClaimableToken {
    pub nft_mint: Pubkey,
    pub amount: u16,
}

#[account]
#[derive(Default)]
pub struct UserStakingAccount {
    pub index: u32,
    pub wallet: Pubkey,
    pub nft_mint_keys: Vec<Pubkey>,
    pub claimable: Vec<ClaimableToken>,
    pub staking_at: u64,
    pub staking_period: u64,
    pub claimable_aury_amount: u64,
    pub aury_deposit: u64,
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
    #[msg("Metadata doesn't exist")]
    MetadataDoesntExist, // 6010, 0x177a
    #[msg("Derived key invalid")]
    DerivedKeyInvalid, // 6011, 0x177b
    #[msg("Invalid accounts")]
    InvalidAccounts, // 6012, 0x177c
    #[msg("Initialize token account failed")]
    InitializeTokenAccountFailed, // 6013, 0x177d
    #[msg("Set account authority failed")]
    SetAccountAuthorityFailed, // 6014, 0x177e
    #[msg("Invalid staking period")]
    InvalidStakingPeriod, // 6015, 0x177f
    #[msg("Staking locked")]
    StakingLocked, // 6016, 0x1780
    #[msg("Staking not locked")]
    StakingNotLocked, // 6017, 0x1781
    #[msg("Incorrect owner")]
    IncorrectOwner, // 6018, 0x1782
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
