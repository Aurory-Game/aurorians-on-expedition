import * as anchor from '@project-serum/anchor';
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token';
import assert from 'assert';
import { nft_data, nft_json_url } from './data';
import {
  createMint,
  setMintAuthority,
  mintToAccount,
  createTokenAccount,
} from './utils';
import fs from 'fs';
import { PublicKey } from '@solana/web3.js';

// manually loading the idl as accessing anchor.workspace
// trigers an error because metadata and vault program don't have idls
const filepath = 'target/idl/nft_staking.json';
const idlStr = fs.readFileSync(filepath);
const idl = JSON.parse(idlStr.toString());

const envProvider = anchor.Provider.env();

let provider = envProvider;

let program;
function setProvider(p: anchor.Provider) {
  provider = p;
  anchor.setProvider(p);
  program = new anchor.Program(idl, idl.metadata.address, p);
}
setProvider(provider);

describe('nft-staking', () => {
  //the program's account for stored initializer key
  let stakingPubkey: PublicKey;
  let stakingBump;
  let nftVaultPubkey: PublicKey;
  let nftVaultBump;

  //winner
  let winner = provider.wallet.publicKey;
  let userStakingPubkey: PublicKey;
  let userStakingBump;
  let userNFTTokenAccount: PublicKey;
  let userRewardTokenAccount: PublicKey[] = [];

  //authorized info
  let authorizedCreator = provider.wallet.publicKey;
  let authorizedNameStarts = ['Helios'];

  //nft mint and metadata
  let mintPubkey: PublicKey;
  let metadataPubkey: PublicKey;

  //reward mint and metadata
  let rewardMintPubkey: PublicKey[] = [];
  let rewardMetadataPubkey: PublicKey[] = [];
  let rewardToken: Token[] = [];

  it('Prepare Mint NFT', async () => {
    const data = nft_data(provider.wallet.publicKey);
    const json_url = nft_json_url;
    const lamports = await Token.getMinBalanceRentForExemptMint(
      provider.connection
    );
    const [mint, metadataPDA, tx] = await createMint(
      provider.wallet.publicKey,
      provider.wallet.publicKey,
      lamports,
      data,
      json_url
    );
    const signers = [mint];

    await provider.send(tx, signers);

    mintPubkey = mint.publicKey;
    metadataPubkey = metadataPDA;
  });

  it('Mint NFT', async () => {
    userNFTTokenAccount = await createTokenAccount(
      provider,
      mintPubkey,
      provider.wallet.publicKey
    );

    await mintToAccount(provider, mintPubkey, userNFTTokenAccount, 1);
  });

  it('Is initialized!', async () => {
    [stakingPubkey, stakingBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(anchor.utils.bytes.utf8.encode('nft_staking'))],
        program.programId
      );

    await program.rpc.initialize(
      stakingBump,
      authorizedCreator,
      authorizedNameStarts,
      {
        accounts: {
          stakingAccount: stakingPubkey,
          initializer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
      }
    );
  });

  it('Prepare Reward Mint NFT', async () => {
    const data = nft_data(provider.wallet.publicKey);
    const json_url = nft_json_url;
    const lamports = await Token.getMinBalanceRentForExemptMint(
      provider.connection
    );

    //// 0
    const [mint0, metadataPDA0, tx0] = await createMint(
      provider.wallet.publicKey,
      provider.wallet.publicKey,
      lamports,
      data,
      json_url
    );
    const signers0 = [mint0];
    await provider.send(tx0, signers0);

    rewardMintPubkey.push(mint0.publicKey);
    rewardMetadataPubkey.push(metadataPDA0);
    rewardToken.push(
      new Token(provider.connection, mint0.publicKey, TOKEN_PROGRAM_ID, null)
    );

    await setMintAuthority(provider, mint0.publicKey, stakingPubkey);

    //// 1
    const [mint1, metadataPDA1, tx1] = await createMint(
      provider.wallet.publicKey,
      provider.wallet.publicKey,
      lamports,
      data,
      json_url
    );
    const signers1 = [mint1];
    await provider.send(tx1, signers1);

    rewardMintPubkey.push(mint1.publicKey);
    rewardMetadataPubkey.push(metadataPDA1);
    rewardToken.push(
      new Token(provider.connection, mint1.publicKey, TOKEN_PROGRAM_ID, null)
    );

    await setMintAuthority(provider, mint1.publicKey, stakingPubkey);

    //// 2
    const [mint2, metadataPDA2, tx2] = await createMint(
      provider.wallet.publicKey,
      provider.wallet.publicKey,
      lamports,
      data,
      json_url
    );
    const signers2 = [mint2];
    await provider.send(tx2, signers2);

    rewardMintPubkey.push(mint2.publicKey);
    rewardMetadataPubkey.push(metadataPDA2);
    rewardToken.push(
      new Token(provider.connection, mint2.publicKey, TOKEN_PROGRAM_ID, null)
    );

    await setMintAuthority(provider, mint2.publicKey, stakingPubkey);

    //// Create User Reward Token Account
    userRewardTokenAccount.push(
      await createTokenAccount(
        provider,
        mint0.publicKey,
        provider.wallet.publicKey
      )
    );
    userRewardTokenAccount.push(
      await createTokenAccount(
        provider,
        mint1.publicKey,
        provider.wallet.publicKey
      )
    );
    userRewardTokenAccount.push(
      await createTokenAccount(
        provider,
        mint2.publicKey,
        provider.wallet.publicKey
      )
    );
  });

  it('Add reward', async () => {
    await program.rpc.addReward(stakingBump, rewardMintPubkey, {
      accounts: {
        stakingAccount: stakingPubkey,
        admin: provider.wallet.publicKey,
      },
    });

    const rewardToken0Info = await rewardToken[0].getMintInfo();
    assert.strictEqual(
      rewardToken0Info.mintAuthority.toString(),
      stakingPubkey.toString()
    );

    const rewardToken1Info = await rewardToken[1].getMintInfo();
    assert.strictEqual(
      rewardToken1Info.mintAuthority.toString(),
      stakingPubkey.toString()
    );

    const rewardToken2Info = await rewardToken[2].getMintInfo();
    assert.strictEqual(
      rewardToken2Info.mintAuthority.toString(),
      stakingPubkey.toString()
    );

    const stakingAccount = await program.account.stakingAccount.fetch(
      stakingPubkey
    );
    assert.equal(
      stakingAccount.activeRewards.toString(),
      rewardMintPubkey.toString()
    );
  });

  it('Remove reward', async () => {
    await program.rpc.removeReward(stakingBump, {
      accounts: {
        stakingAccount: stakingPubkey,
        nftMint: rewardMintPubkey[2],
        nftMintAuthorityTo: provider.wallet.publicKey,
        admin: provider.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    const rewardToken2Info = await rewardToken[2].getMintInfo();
    assert.strictEqual(
      rewardToken2Info.mintAuthority.toString(),
      provider.wallet.publicKey.toString()
    );

    const stakingAccount = await program.account.stakingAccount.fetch(
      stakingPubkey
    );
    assert.equal(
      stakingAccount.activeRewards.toString(),
      [rewardMintPubkey[0], rewardMintPubkey[1]].toString()
    );
  });

  it('Add/Remove authorized name starts', async () => {
    let newAuthorizedNameStarts = ['ABC'];

    // Add
    await program.rpc.addAuthorizedNameStarts(
      stakingBump,
      newAuthorizedNameStarts,
      {
        accounts: {
          stakingAccount: stakingPubkey,
          admin: provider.wallet.publicKey,
        },
      }
    );

    const stakingAccount0 = await program.account.stakingAccount.fetch(
      stakingPubkey
    );
    assert.equal(
      stakingAccount0.authorizedNameStarts.toString(),
      [...authorizedNameStarts, ...newAuthorizedNameStarts].toString()
    );

    // Remove
    await program.rpc.removeAuthorizedNameStarts(
      stakingBump,
      newAuthorizedNameStarts,
      {
        accounts: {
          stakingAccount: stakingPubkey,
          admin: provider.wallet.publicKey,
        },
      }
    );

    const stakingAccount1 = await program.account.stakingAccount.fetch(
      stakingPubkey
    );
    assert.equal(
      stakingAccount1.authorizedNameStarts.toString(),
      authorizedNameStarts.toString()
    );
  });

  it('Update fake authorized creator', async () => {
    let newAuthorizedCreator = new PublicKey(
      '2j85gueUvAFeFEdKZE5yKAvyAsU8fKKZvxX8zLbX8GCc'
    );

    await program.rpc.updateAuthorizedCreator(
      stakingBump,
      newAuthorizedCreator,
      {
        accounts: {
          stakingAccount: stakingPubkey,
          admin: provider.wallet.publicKey,
        },
      }
    );

    const stakingAccount = await program.account.stakingAccount.fetch(
      stakingPubkey
    );
    assert.equal(
      stakingAccount.authorizedCreator.toString(),
      newAuthorizedCreator.toString()
    );
  });

  it('Stake failed with mis-match NFT', async () => {
    [nftVaultPubkey, nftVaultBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [mintPubkey.toBuffer()],
        program.programId
      );
    [userStakingPubkey, userStakingBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [provider.wallet.publicKey.toBuffer()],
        program.programId
      );

    await assert.rejects(
      async () => {
        await program.rpc.stake(nftVaultBump, stakingBump, userStakingBump, {
          accounts: {
            nftMint: mintPubkey,
            nftMetadata: metadataPubkey,
            nftFrom: userNFTTokenAccount,
            nftFromAuthority: provider.wallet.publicKey,
            nftVault: nftVaultPubkey,
            stakingAccount: stakingPubkey,
            userStakingAccount: userStakingPubkey,
            systemProgram: anchor.web3.SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          },
        });
      },
      {
        message:
          'failed to send transaction: Transaction simulation failed: Error processing Instruction 0: custom program error: 0x1772',
      }
    );
  });

  it('Update truth authorized creator', async () => {
    await program.rpc.updateAuthorizedCreator(stakingBump, authorizedCreator, {
      accounts: {
        stakingAccount: stakingPubkey,
        admin: provider.wallet.publicKey,
      },
    });

    const stakingAccount = await program.account.stakingAccount.fetch(
      stakingPubkey
    );
    assert.equal(
      stakingAccount.authorizedCreator.toString(),
      authorizedCreator.toString()
    );
  });

  it('Stake success with match NFT', async () => {
    assert.equal(await getTokenBalance(userNFTTokenAccount), 1);

    [nftVaultPubkey, nftVaultBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [mintPubkey.toBuffer()],
        program.programId
      );
    [userStakingPubkey, userStakingBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [provider.wallet.publicKey.toBuffer()],
        program.programId
      );

    await program.rpc.stake(nftVaultBump, stakingBump, userStakingBump, {
      accounts: {
        nftMint: mintPubkey,
        nftMetadata: metadataPubkey,
        nftFrom: userNFTTokenAccount,
        nftFromAuthority: provider.wallet.publicKey,
        nftVault: nftVaultPubkey,
        stakingAccount: stakingPubkey,
        userStakingAccount: userStakingPubkey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
    });

    assert.equal(await getTokenBalance(userNFTTokenAccount), 0);
    assert.equal(await getTokenBalance(nftVaultPubkey), 1);

    const userStakingAccount = await program.account.userStakingAccount.fetch(
      userStakingPubkey
    );
    assert.equal(
      userStakingAccount.nftMintKeys.toString(),
      [mintPubkey].toString()
    );
  });

  it('Add winner failed with not reward nft', async () => {
    await assert.rejects(
      async () => {
        await program.rpc.addWinner(stakingBump, userStakingBump, winner, {
          accounts: {
            nftMint: rewardMintPubkey[2],
            stakingAccount: stakingPubkey,
            userStakingAccount: userStakingPubkey,
            admin: provider.wallet.publicKey,
          },
        });
      },
      {
        message:
          'failed to send transaction: Transaction simulation failed: Error processing Instruction 0: custom program error: 0x1771',
      }
    );
  });

  it('Add winner success with right reward nft', async () => {
    await program.rpc.addWinner(stakingBump, userStakingBump, winner, {
      accounts: {
        nftMint: rewardMintPubkey[0],
        stakingAccount: stakingPubkey,
        userStakingAccount: userStakingPubkey,
        admin: provider.wallet.publicKey,
      },
    });
    await program.rpc.addWinner(stakingBump, userStakingBump, winner, {
      accounts: {
        nftMint: rewardMintPubkey[0],
        stakingAccount: stakingPubkey,
        userStakingAccount: userStakingPubkey,
        admin: provider.wallet.publicKey,
      },
    });
    await program.rpc.addWinner(stakingBump, userStakingBump, winner, {
      accounts: {
        nftMint: rewardMintPubkey[1],
        stakingAccount: stakingPubkey,
        userStakingAccount: userStakingPubkey,
        admin: provider.wallet.publicKey,
      },
    });

    const userStakingAccount = await program.account.userStakingAccount.fetch(
      userStakingPubkey
    );
    assert.equal(
      userStakingAccount.claimable.toString(),
      [
        { nftMint: rewardMintPubkey[0], amount: 2 },
        { nftMint: rewardMintPubkey[1], amount: 1 },
      ].toString()
    );
  });

  it('Unstake failed before claim', async () => {
    await assert.rejects(
      async () => {
        await program.rpc.unstake(nftVaultBump, stakingBump, userStakingBump, {
          accounts: {
            nftMint: mintPubkey,
            nftTo: userNFTTokenAccount,
            nftToAuthority: provider.wallet.publicKey,
            nftVault: nftVaultPubkey,
            stakingAccount: stakingPubkey,
            userStakingAccount: userStakingPubkey,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
        });
      },
      {
        message:
          'failed to send transaction: Transaction simulation failed: Error processing Instruction 0: custom program error: 0x1778',
      }
    );
  });

  it('Claim failed for not reward', async () => {
    await assert.rejects(
      async () => {
        await program.rpc.claim(stakingBump, userStakingBump, {
          accounts: {
            nftMint: rewardMintPubkey[2],
            nftTo: userRewardTokenAccount[2],
            nftToAuthority: provider.wallet.publicKey,
            stakingAccount: stakingPubkey,
            userStakingAccount: userStakingPubkey,
            tokenProgram: TOKEN_PROGRAM_ID,
          },
        });
      },
      {
        message:
          'failed to send transaction: Transaction simulation failed: Error processing Instruction 0: custom program error: 0x1777',
      }
    );
  });

  it('Claim the reward', async () => {
    await program.rpc.claim(stakingBump, userStakingBump, {
      accounts: {
        nftMint: rewardMintPubkey[0],
        nftTo: userRewardTokenAccount[0],
        nftToAuthority: provider.wallet.publicKey,
        stakingAccount: stakingPubkey,
        userStakingAccount: userStakingPubkey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });
    await program.rpc.claim(stakingBump, userStakingBump, {
      accounts: {
        nftMint: rewardMintPubkey[1],
        nftTo: userRewardTokenAccount[1],
        nftToAuthority: provider.wallet.publicKey,
        stakingAccount: stakingPubkey,
        userStakingAccount: userStakingPubkey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    assert.equal(await getTokenBalance(userRewardTokenAccount[0]), 2);
    assert.equal(await getTokenBalance(userRewardTokenAccount[1]), 1);
  });

  it('Unstake success after claim', async () => {
    await program.rpc.unstake(nftVaultBump, stakingBump, userStakingBump, {
      accounts: {
        nftMint: mintPubkey,
        nftTo: userNFTTokenAccount,
        nftToAuthority: provider.wallet.publicKey,
        nftVault: nftVaultPubkey,
        stakingAccount: stakingPubkey,
        userStakingAccount: userStakingPubkey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    assert.equal(await getTokenBalance(userNFTTokenAccount), 1);

    const userStakingAccount = await program.account.userStakingAccount.fetch(
      userStakingPubkey
    );
    assert.equal(userStakingAccount.nftMintKeys.toString(), [].toString());

    const mintToken = new Token(
      provider.connection,
      mintPubkey,
      TOKEN_PROGRAM_ID,
      null
    );

    await assert.rejects(
      async () => {
        await mintToken.getAccountInfo(nftVaultPubkey);
      },
      {
        message: 'Failed to find account',
      }
    );
  });

  it('Mint to', async () => {
    await program.rpc.mintTo(stakingBump, new anchor.BN(2), {
      accounts: {
        nftMint: rewardMintPubkey[1],
        nftTo: userRewardTokenAccount[1],
        stakingAccount: stakingPubkey,
        admin: provider.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
    });

    assert.equal(await getTokenBalance(userRewardTokenAccount[1]), 3);
  });
});

async function getTokenBalance(pubkey: PublicKey) {
  return parseInt(
    (await provider.connection.getTokenAccountBalance(pubkey)).value.amount
  );
}
