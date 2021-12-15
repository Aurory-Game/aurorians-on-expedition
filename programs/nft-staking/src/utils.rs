use {
    crate::ErrorCode,
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

#[inline(always)]
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

#[inline(always)]
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
