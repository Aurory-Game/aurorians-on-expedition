use anchor_lang::prelude::*;
use metaplex_token_metadata::state::Metadata;
use spl_token::instruction::AuthorityType;

use {
    crate::*,
    anchor_lang::{
        prelude::{AccountInfo, ProgramResult},
        solana_program::program::invoke_signed,
    },
};

///TokenTransferParams
pub struct TokenTransferParams<'a: 'b, 'b> {
    /// source
    pub source: AccountInfo<'a>,
    /// destination
    pub destination: AccountInfo<'a>,
    /// amount
    pub amount: u64,
    /// authority
    pub authority: AccountInfo<'a>,
    /// authority_signer_seeds
    pub authority_signer_seeds: &'b [&'b [u8]],
    /// token_program
    pub token_program: AccountInfo<'a>,
}

///TokenMintParams
pub struct TokenMintParams<'a: 'b, 'b> {
    /// mint
    pub mint: AccountInfo<'a>,
    /// to
    pub to: AccountInfo<'a>,
    /// amount
    pub amount: u64,
    /// owner
    pub owner: AccountInfo<'a>,
    /// owner_signer_seeds
    pub owner_signer_seeds: &'b [&'b [u8]],
    /// token_program
    pub token_program: AccountInfo<'a>,
}

///InitializeTokenAccount
pub struct InitializeTokenAccountParams<'a: 'b, 'b> {
    /// account
    pub account: AccountInfo<'a>,
    /// account_signer_seeds
    pub account_signer_seeds: &'b [&'b [u8]],
    /// mint
    pub mint: AccountInfo<'a>,
    /// owner
    pub owner: AccountInfo<'a>,
    /// token_program
    pub token_program: AccountInfo<'a>,
}

///SetAuthority
pub struct SetAuthorityParams<'a: 'b, 'b> {
    /// account
    pub account: AccountInfo<'a>,
    /// new authority
    pub new_authority: AccountInfo<'a>,
    /// authority type
    pub authority_type: AuthorityType,
    /// owner
    pub owner: AccountInfo<'a>,
    /// owner_signer_seeds
    pub owner_signer_seeds: &'b [&'b [u8]],
    /// token_program
    pub token_program: AccountInfo<'a>,
}

///CloseAccountParams
pub struct CloseAccountParams<'a: 'b, 'b> {
    /// account
    pub account: AccountInfo<'a>,
    /// destination
    pub destination: AccountInfo<'a>,
    /// owner
    pub owner: AccountInfo<'a>,
    /// owner_signer_seeds
    pub owner_signer_seeds: &'b [&'b [u8]],
    /// token_program
    pub token_program: AccountInfo<'a>,
}

pub fn spl_token_transfer(params: TokenTransferParams<'_, '_>) -> ProgramResult {
    let TokenTransferParams {
        source,
        destination,
        authority,
        token_program,
        amount,
        authority_signer_seeds,
    } = params;

    let result = invoke_signed(
        &spl_token::instruction::transfer(
            token_program.key,
            source.key,
            destination.key,
            authority.key,
            &[],
            amount,
        )?,
        &[source, destination, authority, token_program],
        &[authority_signer_seeds],
    );

    result.map_err(|_| ErrorCode::TokenTransferFailed.into())
}

pub fn spl_token_mint(params: TokenMintParams<'_, '_>) -> ProgramResult {
    let TokenMintParams {
        mint,
        to,
        amount,
        owner,
        owner_signer_seeds,
        token_program,
    } = params;

    let result = invoke_signed(
        &spl_token::instruction::mint_to(
            token_program.key,
            mint.key,
            to.key,
            owner.key,
            &[],
            amount,
        )?,
        &[mint, to, owner, token_program],
        &[owner_signer_seeds],
    );

    result.map_err(|_| ErrorCode::TokenMintFailed.into())
}

pub fn spl_init_token_account(params: InitializeTokenAccountParams<'_, '_>) -> ProgramResult {
    let InitializeTokenAccountParams {
        account,
        account_signer_seeds,
        mint,
        owner,
        token_program,
    } = params;

    let result = invoke_signed(
        &spl_token::instruction::initialize_account(
            token_program.key,
            account.key,
            mint.key,
            owner.key,
        )?,
        &[account, mint, owner, token_program],
        &[account_signer_seeds],
    );

    result.map_err(|_| ErrorCode::InitializeTokenAccountFailed.into())
}

pub fn spl_set_authority(params: SetAuthorityParams<'_, '_>) -> ProgramResult {
    let SetAuthorityParams {
        account,
        new_authority,
        authority_type,
        owner,
        owner_signer_seeds,
        token_program,
    } = params;

    let result = invoke_signed(
        &spl_token::instruction::set_authority(
            token_program.key,
            account.key,
            Some(new_authority.key),
            authority_type,
            owner.key,
            &[],
        )?,
        &[account, new_authority, owner, token_program],
        &[owner_signer_seeds],
    );

    result.map_err(|_| ErrorCode::SetAccountAuthorityFailed.into())
}

pub fn spl_close_account(params: CloseAccountParams<'_, '_>) -> ProgramResult {
    let CloseAccountParams {
        account,
        destination,
        owner,
        owner_signer_seeds,
        token_program,
    } = params;

    let result = invoke_signed(
        &spl_token::instruction::close_account(
            token_program.key,
            account.key,
            destination.key,
            owner.key,
            &[],
        )?,
        &[account, destination, owner, token_program],
        &[owner_signer_seeds],
    );

    result.map_err(|_| ErrorCode::CloseAccountFailed.into())
}

pub fn assert_metadata_valid<'info>(
    nft_metadata: &AccountInfo,
    mint: &Pubkey,
    staking_account: Box<Account<StakingAccount>>,
    program_id: &Pubkey,
) -> ProgramResult {
    // determine metaplex program id
    assert_owned_by(nft_metadata, program_id)?;

    // determine metadata mint
    assert_derivation(
        &metaplex_token_metadata::id(),
        nft_metadata,
        &[
            metaplex_token_metadata::state::PREFIX.as_bytes(),
            metaplex_token_metadata::id().as_ref(),
            mint.as_ref(),
        ],
    )?;
    if nft_metadata.data_is_empty() {
        return Err(ErrorCode::MetadataDoesntExist.into());
    }

    let metadata = Metadata::from_account_info(&nft_metadata)?;

    // determine authorized name start
    if staking_account
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

    // determine authorized creator
    match metadata.data.creators {
        Some(creators) => {
            // determine authorized creator
            if creators.iter().find(|&creator| {
                creator.verified && creator.address == staking_account.authorized_creator
            }) == None
            {
                return Err(ErrorCode::NoAuthorizedCreatorsFoundInMetadata.into());
            }

            return Ok(());
        }
        None => {
            return Err(ErrorCode::NoAuthorizedCreatorsFoundInMetadata.into());
        }
    };
}

pub fn assert_derivation(program_id: &Pubkey, account: &AccountInfo, path: &[&[u8]]) -> Result<u8> {
    let (key, bump) = Pubkey::find_program_address(&path, program_id);
    if key != *account.key {
        return Err(ErrorCode::DerivedKeyInvalid.into());
    }
    Ok(bump)
}

pub fn assert_owned_by(account: &AccountInfo, owner: &Pubkey) -> ProgramResult {
    if account.owner != owner {
        Err(MetadataError::IncorrectOwner.into())
    } else {
        Ok(())
    }
}
