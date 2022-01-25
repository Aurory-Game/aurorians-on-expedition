import * as anchor from '@project-serum/anchor';
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token';
import assert from 'assert';
import { expect } from 'chai';
import { nft_data, nft_json_url } from './data';
import {
  createMint,
  setMintAuthority,
  mintToAccount,
  createTokenAccount,
  createTokenMint,
} from './utils';
import fs from 'fs';
import dayjs from 'dayjs';
import { PublicKey } from '@solana/web3.js';
import { Program } from '@project-serum/anchor';
import { NftStaking } from '../target/types/nft_staking';
import { numberEnumValues } from 'quicktype-core/dist/support/Support';

// manually loading the idl as accessing anchor.workspace
// trigers an error because metadata and vault program don't have idls
const filepath = 'target/idl/nft_staking.json';
const idlStr = fs.readFileSync(filepath);
const idl = JSON.parse(idlStr.toString());

const envProvider = anchor.Provider.env();

let provider = envProvider;

let program: Program<NftStaking>;
function setProvider(p: anchor.Provider) {
  provider = p;
  anchor.setProvider(p);
  program = new anchor.Program(
    idl,
    idl.metadata.address,
    p
  ) as Program<NftStaking>;
}
setProvider(provider);

describe('nft-staking', () => {
  //the program's account for stored initializer key
  let stakingPubkey: PublicKey;
  let stakingBump: number;

  let auryToken: Token;
  let auryMintPubkey: PublicKey;
  let auryVaultPubkey: PublicKey;
  let auryVaultBump: number;

  let nftVaultPubkey: PublicKey[] = [];
  let nftVaultBump: number[] = [];

  let minimumStakingPeriod = new anchor.BN(1);
  let maximumStakingPeriod = new anchor.BN(20);

  //winner
  let winner = provider.wallet.publicKey;

  let userStakingCounterPubkey: PublicKey;
  let userStakingCounterBump: number;

  let userStakingPubkey: PublicKey;
  let userStakingBump: number;
  let userStakingIndex = 0;
  let userStakingPeriod = new anchor.BN(2);

  let nextUserStakingPubkey: PublicKey;
  let nextUserStakingBump: number;
  let nextUserStakingIndex = 1;

  let userAuryTokenAccount: PublicKey;
  let userNFTTokenAccount: PublicKey[] = [];
  let userRewardTokenAccount: PublicKey[] = [];
  let userAuryRewardAmount = new anchor.BN(100);

  //authorized info
  let authorizedCreator = provider.wallet.publicKey;
  let fakeAuthorizedCreator = new PublicKey(
    '6EcQDdBqyqDLkdbF8cKusnPrZGsAEhWLyXyvATDwXt2L'
  );
  let authorizedNameStarts = ['Helios', 'Crystal', 'Axe', 'Sven', 'Tinker'];

  //nft mint and metadata
  let nftMintPubkey: PublicKey[] = [];
  let nftMetadataPubkey: PublicKey[] = [];
  let nftToken: Token[] = [];
  let nftCount = 4;

  //reward mint and metadata
  let rewardMintPubkey: PublicKey[] = [];
  let rewardMetadataPubkey: PublicKey[] = [];
  let rewardToken: Token[] = [];
  let rewardCount = 2;

  let notRewardMintPubkey: PublicKey;

  it('Prepare Aury', async () => {
    // Aury MintAccount
    const rawData = fs.readFileSync(
      'tests/keys/aury-teST1ieLrLdr4MJPZ7i8mgSCLQ7rTrPRjNnyFdHFaz9.json'
    );
    const keyData = JSON.parse(rawData.toString());
    const mintKey = anchor.web3.Keypair.fromSecretKey(new Uint8Array(keyData));
    auryToken = await createTokenMint(
      provider,
      mintKey,
      provider.wallet.publicKey,
      null,
      9,
      TOKEN_PROGRAM_ID
    );

    // User Aury TokenAccount and Mint 1000 AURY
    auryMintPubkey = auryToken.publicKey;
    userAuryTokenAccount = await createTokenAccount(
      provider,
      auryMintPubkey,
      provider.wallet.publicKey
    );
    await mintToAccount(provider, auryMintPubkey, userAuryTokenAccount, 1000);
  });

  it('Prepare NFT that will be staked', async () => {
    for (let i = 0; i < nftCount; i++) {
      // NFT MintAccount
      const data = nft_data(
        provider.wallet.publicKey,
        authorizedNameStarts[i] + ': #' + (i + 1).toString()
      );
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

      nftMintPubkey.push(mint.publicKey);
      nftMetadataPubkey.push(metadataPDA);
      nftToken.push(
        new Token(provider.connection, mint.publicKey, TOKEN_PROGRAM_ID, null)
      );

      // User NFT TokenAccount and Mint 1~5 NFTs
      const tokenAccount = await createTokenAccount(
        provider,
        mint.publicKey,
        provider.wallet.publicKey
      );
      await mintToAccount(provider, mint.publicKey, tokenAccount, i + 1);
      userNFTTokenAccount.push(tokenAccount);

      // NFT vault pda
      let [pubkey, bump] = await anchor.web3.PublicKey.findProgramAddress(
        [provider.wallet.publicKey.toBuffer(), mint.publicKey.toBuffer()],
        program.programId
      );

      nftVaultPubkey.push(pubkey);
      nftVaultBump.push(bump);
    }
  });

  it('Prepare staking & aury_vault pda', async () => {
    [stakingPubkey, stakingBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(anchor.utils.bytes.utf8.encode('nft_staking'))],
        program.programId
      );
    [auryVaultPubkey, auryVaultBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [auryMintPubkey.toBuffer()],
        program.programId
      );
  });

  it('Prepare Reward Mint NFT ', async () => {
    for (let i = 0; i <= rewardCount; i++) {
      // Reward NFT MintAccount
      const data = nft_data(
        provider.wallet.publicKey,
        'Reward' + ': #' + (i + 1).toString()
      );
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

      rewardMintPubkey.push(mint.publicKey);
      rewardMetadataPubkey.push(metadataPDA);
      rewardToken.push(
        new Token(provider.connection, mint.publicKey, TOKEN_PROGRAM_ID, null)
      );

      // Reward NFT Mint Authority to the program staking pda
      await setMintAuthority(provider, mint.publicKey, stakingPubkey);

      // User Reward NFT TokenAccount
      userRewardTokenAccount.push(
        await createTokenAccount(
          provider,
          mint.publicKey,
          provider.wallet.publicKey
        )
      );
    }
  });

  it('Is initialized!', async () => {
    await program.rpc.initialize(
      stakingBump,
      auryVaultBump,
      authorizedCreator,
      authorizedNameStarts,
      minimumStakingPeriod,
      maximumStakingPeriod,
      {
        accounts: {
          stakingAccount: stakingPubkey,
          auryMint: auryMintPubkey,
          auryVault: auryVaultPubkey,
          initializer: provider.wallet.publicKey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
      }
    );

    const stakingAccount = await program.account.stakingAccount.fetch(
      stakingPubkey
    );
    assert.equal(
      stakingAccount.adminKey.toString(),
      provider.wallet.publicKey.toString()
    );
    assert.equal(
      stakingAccount.authorizedCreator.toString(),
      authorizedCreator.toString()
    );
    assert.equal(
      stakingAccount.authorizedNameStarts.toString(),
      authorizedNameStarts.toString()
    );
    assert.equal(
      stakingAccount.minimumStakingPeriod.toNumber(),
      minimumStakingPeriod.toNumber()
    );
    assert.equal(
      stakingAccount.maximumStakingPeriod.toNumber(),
      maximumStakingPeriod.toNumber()
    );
  });

  it('Add reward', async () => {
    await program.rpc.addReward(stakingBump, rewardMintPubkey, {
      accounts: {
        stakingAccount: stakingPubkey,
        admin: provider.wallet.publicKey,
      },
    });

    for (let i = 0; i <= rewardCount; i++) {
      const rewardTokenInfo = await rewardToken[i].getMintInfo();
      assert.strictEqual(
        rewardTokenInfo.mintAuthority.toString(),
        stakingPubkey.toString()
      );
    }

    const stakingAccount = await program.account.stakingAccount.fetch(
      stakingPubkey
    );
    assert.equal(
      stakingAccount.activeRewards.toString(),
      rewardMintPubkey.toString()
    );
  });

  it('Remove reward', async () => {
    // Remaining accounts - mint(writable)
    let remainingAccounts = [
      {
        pubkey: rewardMintPubkey[rewardCount],
        isWritable: true,
        isSigner: false,
      },
    ];

    await program.rpc.removeReward(stakingBump, {
      accounts: {
        stakingAccount: stakingPubkey,
        nftMintAuthorityTo: provider.wallet.publicKey,
        admin: provider.wallet.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      remainingAccounts,
    });

    const rewardTokenInfo = await rewardToken[rewardCount].getMintInfo();
    assert.strictEqual(
      rewardTokenInfo.mintAuthority.toString(),
      provider.wallet.publicKey.toString()
    );

    notRewardMintPubkey = rewardMintPubkey.pop();
    rewardToken.pop();

    const stakingAccount = await program.account.stakingAccount.fetch(
      stakingPubkey
    );
    assert.equal(
      stakingAccount.activeRewards.toString(),
      rewardMintPubkey.toString()
    );
  });

  it('Add/Remove authorized name starts', async () => {
    let newAuthorizedNameStarts = ['ABC', 'DEF'];

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

  it('Update staking period', async () => {
    let newMinimumStakingPeriod = new anchor.BN(5);
    let newMaximumStakingPeriod = new anchor.BN(10);

    await program.rpc.updateStakingPeriod(
      stakingBump,
      newMinimumStakingPeriod,
      newMaximumStakingPeriod,
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
      stakingAccount.minimumStakingPeriod.toNumber(),
      newMinimumStakingPeriod.toNumber()
    );
    assert.equal(
      stakingAccount.maximumStakingPeriod.toNumber(),
      newMaximumStakingPeriod.toNumber()
    );

    await program.rpc.updateStakingPeriod(
      stakingBump,
      minimumStakingPeriod,
      maximumStakingPeriod,
      {
        accounts: {
          stakingAccount: stakingPubkey,
          admin: provider.wallet.publicKey,
        },
      }
    );
  });

  it('Update fake authorized creator', async () => {
    await program.rpc.updateAuthorizedCreator(
      stakingBump,
      fakeAuthorizedCreator,
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
      fakeAuthorizedCreator.toString()
    );
  });

  it('Stake failed with mis-match NFT', async () => {
    // UserStakingCounterAccount pda
    [userStakingCounterPubkey, userStakingCounterBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [provider.wallet.publicKey.toBuffer()],
        program.programId
      );

    // UserStakingAccount pda
    [userStakingPubkey, userStakingBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from(
            anchor.utils.bytes.utf8.encode(
              new anchor.BN(userStakingIndex).toString()
            )
          ),
          provider.wallet.publicKey.toBuffer(),
        ],
        program.programId
      );

    // nftVaultBumps
    let nftVaultBumps = Buffer.from([nftVaultBump[0]]);

    // Remaining accounts - mint(readonly), metadata(readonly), tokenAccount(writable), vault(writable)
    let remainingAccounts = [
      {
        pubkey: nftMintPubkey[0],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: nftMetadataPubkey[0],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[0],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[0],
        isWritable: true,
        isSigner: false,
      },
    ];

    await assert.rejects(
      async () => {
        await program.rpc.stake(
          nftVaultBumps,
          stakingBump,
          userStakingCounterBump,
          userStakingBump,
          {
            accounts: {
              nftFromAuthority: provider.wallet.publicKey,
              stakingAccount: stakingPubkey,
              userStakingCounterAccount: userStakingCounterPubkey,
              userStakingAccount: userStakingPubkey,
              systemProgram: anchor.web3.SystemProgram.programId,
              tokenProgram: TOKEN_PROGRAM_ID,
              rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            },
            remainingAccounts,
          }
        );
      },
      {
        code: 6002,
        // message: '6002: No authorized creators found in metadata',
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

  it('Stake failed with mis-match mint & metadata', async () => {
    // nftVaultBumps
    let nftVaultBumps = Buffer.from([nftVaultBump[0]]);

    // Remaining accounts - mint(readonly), metadata(readonly), tokenAccount(writable), vault(writable)
    let remainingAccounts = [
      {
        pubkey: nftMintPubkey[0],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: rewardMetadataPubkey[0],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[0],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[0],
        isWritable: true,
        isSigner: false,
      },
    ];

    await assert.rejects(
      async () => {
        await program.rpc.stake(
          nftVaultBumps,
          stakingBump,
          userStakingCounterBump,
          userStakingBump,
          {
            accounts: {
              nftFromAuthority: provider.wallet.publicKey,
              stakingAccount: stakingPubkey,
              userStakingCounterAccount: userStakingCounterPubkey,
              userStakingAccount: userStakingPubkey,
              systemProgram: anchor.web3.SystemProgram.programId,
              tokenProgram: TOKEN_PROGRAM_ID,
              rent: anchor.web3.SYSVAR_RENT_PUBKEY,
            },
            remainingAccounts,
          }
        );
      },
      {
        code: 6011,
        // message: '6011: Derived key invalid',
      }
    );
  });

  it('Stake success with match NFT - 0, 1', async () => {
    // nft balance of user
    assert.equal(await getTokenBalance(userNFTTokenAccount[0]), 1);
    assert.equal(await getTokenBalance(userNFTTokenAccount[1]), 2);

    // nftVaultBumps
    let nftVaultBumps = Buffer.from([nftVaultBump[0], nftVaultBump[1]]);

    // Remaining accounts - mint(readonly), metadata(readonly), tokenAccount(writable), vault(writable)
    let remainingAccounts = [
      {
        pubkey: nftMintPubkey[0],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: nftMetadataPubkey[0],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[0],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[0],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftMintPubkey[1],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: nftMetadataPubkey[1],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[1],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[1],
        isWritable: true,
        isSigner: false,
      },
    ];

    await program.rpc.stake(
      nftVaultBumps,
      stakingBump,
      userStakingCounterBump,
      userStakingBump,
      {
        accounts: {
          nftFromAuthority: provider.wallet.publicKey,
          stakingAccount: stakingPubkey,
          userStakingCounterAccount: userStakingCounterPubkey,
          userStakingAccount: userStakingPubkey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        remainingAccounts,
      }
    );

    // nft balance of user and program
    assert.equal(await getTokenBalance(userNFTTokenAccount[0]), 0);
    assert.equal(await getTokenBalance(nftVaultPubkey[0]), 1);
    assert.equal(await getTokenBalance(userNFTTokenAccount[1]), 1);
    assert.equal(await getTokenBalance(nftVaultPubkey[1]), 1);

    // user staking account
    const userStakingAccount = await program.account.userStakingAccount.fetch(
      userStakingPubkey
    );
    assert.equal(
      userStakingAccount.nftMintKeys.toString(),
      [nftMintPubkey[0], nftMintPubkey[1]].toString()
    );
  });

  it('Stake success with match NFT - 1, 2, 3', async () => {
    // nft balance of user
    assert.equal(await getTokenBalance(userNFTTokenAccount[1]), 1);
    assert.equal(await getTokenBalance(userNFTTokenAccount[2]), 3);
    assert.equal(await getTokenBalance(userNFTTokenAccount[3]), 4);

    // nftVaultBumps
    let nftVaultBumps = Buffer.from([
      nftVaultBump[1],
      nftVaultBump[2],
      nftVaultBump[3],
    ]);

    // Remaining accounts - mint(readonly), metadata(readonly), tokenAccount(writable), vault(writable)
    let remainingAccounts = [
      {
        pubkey: nftMintPubkey[1],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: nftMetadataPubkey[1],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[1],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[1],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftMintPubkey[2],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: nftMetadataPubkey[2],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[2],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[2],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftMintPubkey[3],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: nftMetadataPubkey[3],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[3],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[3],
        isWritable: true,
        isSigner: false,
      },
    ];

    await program.rpc.stake(
      nftVaultBumps,
      stakingBump,
      userStakingCounterBump,
      userStakingBump,
      {
        accounts: {
          nftFromAuthority: provider.wallet.publicKey,
          stakingAccount: stakingPubkey,
          userStakingCounterAccount: userStakingCounterPubkey,
          userStakingAccount: userStakingPubkey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        remainingAccounts,
      }
    );

    // nft balance of user and program
    assert.equal(await getTokenBalance(userNFTTokenAccount[1]), 0);
    assert.equal(await getTokenBalance(nftVaultPubkey[1]), 2);
    assert.equal(await getTokenBalance(userNFTTokenAccount[2]), 2);
    assert.equal(await getTokenBalance(nftVaultPubkey[2]), 1);
    assert.equal(await getTokenBalance(userNFTTokenAccount[3]), 3);
    assert.equal(await getTokenBalance(nftVaultPubkey[3]), 1);

    // user staking account
    const userStakingAccount = await program.account.userStakingAccount.fetch(
      userStakingPubkey
    );
    assert.equal(
      userStakingAccount.nftMintKeys.toString(),
      [
        nftMintPubkey[0],
        nftMintPubkey[1],
        nftMintPubkey[1],
        nftMintPubkey[2],
        nftMintPubkey[3],
      ].toString()
    );
  });

  it('Unstake failed for not locked staking', async () => {
    await assert.rejects(
      async () => {
        await program.rpc.unstake(
          stakingBump,
          userStakingIndex,
          userStakingBump,
          {
            accounts: {
              nftToAuthority: provider.wallet.publicKey,
              stakingAccount: stakingPubkey,
              userStakingAccount: userStakingPubkey,
              tokenProgram: TOKEN_PROGRAM_ID,
            },
          }
        );
      },
      {
        code: 6017,
        // message: '6017: Staking not locked',
      }
    );
  });

  it('Lock stake failed with invalid staking period', async () => {
    let invalidStakingPeriod = maximumStakingPeriod.add(minimumStakingPeriod);

    await assert.rejects(
      async () => {
        await program.rpc.lockStake(
          stakingBump,
          userStakingCounterBump,
          userStakingBump,
          invalidStakingPeriod,
          {
            accounts: {
              nftFromAuthority: provider.wallet.publicKey,
              stakingAccount: stakingPubkey,
              userStakingCounterAccount: userStakingCounterPubkey,
              userStakingAccount: userStakingPubkey,
            },
          }
        );
      },
      {
        code: 6015,
        // message: '6015: Invalid staking period',
      }
    );
  });

  it('Add winner failed for not locked staking', async () => {
    // Remaining accounts - mint(readonly), userStakingAccount(writable)
    let remainingAccounts = [
      {
        pubkey: rewardMintPubkey[0],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userStakingPubkey,
        isWritable: true,
        isSigner: false,
      },
    ];

    // winnerStakingIndex
    let winnerStakingIndexes = Buffer.from([userStakingIndex]);
    // winner
    let winners = [winner];

    await assert.rejects(
      async () => {
        await program.rpc.addWinner(
          stakingBump,
          winnerStakingIndexes,
          winners,
          {
            accounts: {
              stakingAccount: stakingPubkey,
              admin: provider.wallet.publicKey,
            },
            remainingAccounts,
          }
        );
      },
      {
        code: 6017,
        // message: '6017: Staking not locked',
      }
    );
  });

  it('Add aury winner failed for not locked staking', async () => {
    // Remaining accounts - userStakingAccount(writable)
    let remainingAccounts = [
      {
        pubkey: userStakingPubkey,
        isWritable: true,
        isSigner: false,
      },
    ];

    // winnerStakingIndex
    let winnerStakingIndexes = Buffer.from([userStakingIndex]);
    // winner
    let winners = [winner];
    // auryAmounts
    let auryAmounts = [userAuryRewardAmount];

    await assert.rejects(
      async () => {
        await program.rpc.addAuryWinner(
          stakingBump,
          auryVaultBump,
          winnerStakingIndexes,
          winners,
          auryAmounts,
          {
            accounts: {
              stakingAccount: stakingPubkey,
              auryMint: auryMintPubkey,
              auryVault: auryVaultPubkey,
              auryFrom: userAuryTokenAccount,
              admin: provider.wallet.publicKey,
              tokenProgram: TOKEN_PROGRAM_ID,
            },
            remainingAccounts,
          }
        );
      },
      {
        code: 6017,
        // message: '6017: Staking not locked',
      }
    );
  });

  it('Lock stake success with valid staking period', async () => {
    let userStakingAtFloor = dayjs().unix() - 1;

    await program.rpc.lockStake(
      stakingBump,
      userStakingCounterBump,
      userStakingBump,
      userStakingPeriod,
      {
        accounts: {
          nftFromAuthority: provider.wallet.publicKey,
          stakingAccount: stakingPubkey,
          userStakingCounterAccount: userStakingCounterPubkey,
          userStakingAccount: userStakingPubkey,
        },
      }
    );

    let userStakingAtCeil = dayjs().unix() + 1;

    let userStakingCounterAccount =
      await program.account.userStakingCounterAccount.fetch(
        userStakingCounterPubkey
      );
    assert.equal(userStakingCounterAccount.counter, 1);

    let userStakingAccount = await program.account.userStakingAccount.fetch(
      userStakingPubkey
    );
    assert.equal(
      userStakingAccount.stakingPeriod.toNumber(),
      userStakingPeriod.toNumber()
    );
    expect(userStakingAccount.stakingAt.toNumber()).to.be.at.least(
      userStakingAtFloor
    );
    expect(userStakingAccount.stakingAt.toNumber()).to.be.at.most(
      userStakingAtCeil
    );
  });

  it('Unstake failed for locked staking', async () => {
    await assert.rejects(
      async () => {
        await program.rpc.unstake(
          stakingBump,
          userStakingIndex,
          userStakingBump,
          {
            accounts: {
              nftToAuthority: provider.wallet.publicKey,
              stakingAccount: stakingPubkey,
              userStakingAccount: userStakingPubkey,
              tokenProgram: TOKEN_PROGRAM_ID,
            },
          }
        );
      },
      {
        code: 6016,
        // message: '6016: Staking locked',
      }
    );
  });

  it('Add winner failed with not reward nft', async () => {
    // Remaining accounts - mint(readonly), userStakingAccount(writable)
    let remainingAccounts = [
      {
        pubkey: notRewardMintPubkey,
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userStakingPubkey,
        isWritable: true,
        isSigner: false,
      },
    ];

    // winnerStakingIndex
    let winnerStakingIndexes = Buffer.from([userStakingIndex]);
    // winner
    let winners = [winner];

    await assert.rejects(
      async () => {
        await program.rpc.addWinner(
          stakingBump,
          winnerStakingIndexes,
          winners,
          {
            accounts: {
              stakingAccount: stakingPubkey,
              admin: provider.wallet.publicKey,
            },
            remainingAccounts,
          }
        );
      },
      {
        code: 6001,
        // message: '6001: Invalid mint for reward',
      }
    );
  });

  it('Add winner success with right reward nft', async () => {
    // Remaining accounts - mint(readonly), userStakingAccount(writable)
    let remainingAccounts = [
      {
        pubkey: rewardMintPubkey[0],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userStakingPubkey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: rewardMintPubkey[0],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userStakingPubkey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: rewardMintPubkey[1],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userStakingPubkey,
        isWritable: true,
        isSigner: false,
      },
    ];

    // winnerStakingIndex
    let winnerStakingIndexes = Buffer.from([
      userStakingIndex,
      userStakingIndex,
      userStakingIndex,
    ]);
    // winner
    let winners = [winner, winner, winner];

    await program.rpc.addWinner(stakingBump, winnerStakingIndexes, winners, {
      accounts: {
        stakingAccount: stakingPubkey,
        admin: provider.wallet.publicKey,
      },
      remainingAccounts,
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

  it('Add aury winner success', async () => {
    // Old balances
    let oldBalance = await getTokenBalance(userAuryTokenAccount);
    let oldUserStakingAccount = await program.account.userStakingAccount.fetch(
      userStakingPubkey
    );

    // Remaining accounts - userStakingAccount(writable)
    let remainingAccounts = [
      {
        pubkey: userStakingPubkey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: userStakingPubkey,
        isWritable: true,
        isSigner: false,
      },
    ];

    // winnerStakingIndex
    let winnerStakingIndexes = Buffer.from([
      userStakingIndex,
      userStakingIndex,
    ]);
    // winner
    let winners = [winner, winner];
    // auryAmounts
    let auryAmounts = [
      userAuryRewardAmount.div(new anchor.BN(2)),
      userAuryRewardAmount.div(new anchor.BN(2)),
    ];

    await program.rpc.addAuryWinner(
      stakingBump,
      auryVaultBump,
      winnerStakingIndexes,
      winners,
      auryAmounts,
      {
        accounts: {
          stakingAccount: stakingPubkey,
          auryMint: auryMintPubkey,
          auryVault: auryVaultPubkey,
          auryFrom: userAuryTokenAccount,
          admin: provider.wallet.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
        remainingAccounts,
      }
    );

    // New balances
    let newBalance = await getTokenBalance(userAuryTokenAccount);
    let newUserStakingAccount = await program.account.userStakingAccount.fetch(
      userStakingPubkey
    );

    assert.equal(oldBalance - newBalance, userAuryRewardAmount.toNumber());
    assert.equal(
      newUserStakingAccount.claimableAuryAmount.toNumber() -
        oldUserStakingAccount.claimableAuryAmount.toNumber(),
      userAuryRewardAmount.toNumber()
    );
  });

  it('Unstake failed before claim all rewards', async () => {
    await assert.rejects(
      async () => {
        await program.rpc.unstake(
          stakingBump,
          userStakingIndex,
          userStakingBump,
          {
            accounts: {
              nftToAuthority: provider.wallet.publicKey,
              stakingAccount: stakingPubkey,
              userStakingAccount: userStakingPubkey,
              tokenProgram: TOKEN_PROGRAM_ID,
            },
          }
        );
      },
      {
        code: 6008,
        // message: "6008: Can't unstake before claim all rewards",
      }
    );
  });

  it('Claim failed for not reward', async () => {
    // Remaining accounts - mint(writable), tokenAccount(writable)
    let remainingAccounts = [
      {
        pubkey: notRewardMintPubkey,
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: userRewardTokenAccount[2],
        isWritable: true,
        isSigner: false,
      },
    ];

    await assert.rejects(
      async () => {
        await program.rpc.claim(
          stakingBump,
          userStakingIndex,
          userStakingBump,
          {
            accounts: {
              nftToAuthority: provider.wallet.publicKey,
              stakingAccount: stakingPubkey,
              userStakingAccount: userStakingPubkey,
              tokenProgram: TOKEN_PROGRAM_ID,
            },
            remainingAccounts,
          }
        );
      },
      {
        code: 6007,
        // message: '6007: Not claimable item',
      }
    );
  });

  it('Claim the reward', async () => {
    // Remaining accounts - mint(writable), tokenAccount(writable)
    let remainingAccounts = [
      {
        pubkey: rewardMintPubkey[0],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: userRewardTokenAccount[0],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: rewardMintPubkey[1],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: userRewardTokenAccount[1],
        isWritable: true,
        isSigner: false,
      },
    ];

    await program.rpc.claim(stakingBump, userStakingIndex, userStakingBump, {
      accounts: {
        nftToAuthority: provider.wallet.publicKey,
        stakingAccount: stakingPubkey,
        userStakingAccount: userStakingPubkey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      remainingAccounts,
    });

    assert.equal(await getTokenBalance(userRewardTokenAccount[0]), 2);
    assert.equal(await getTokenBalance(userRewardTokenAccount[1]), 1);
  });

  it('Claim the aury reward', async () => {
    let oldBalance = await getTokenBalance(userAuryTokenAccount);

    await program.rpc.claimAuryReward(
      auryVaultBump,
      userStakingIndex,
      userStakingBump,
      {
        accounts: {
          auryMint: auryMintPubkey,
          auryVault: auryVaultPubkey,
          auryTo: userAuryTokenAccount,
          auryToAuthority: provider.wallet.publicKey,
          userStakingAccount: userStakingPubkey,
          tokenProgram: TOKEN_PROGRAM_ID,
        },
      }
    );

    let newBalance = await getTokenBalance(userAuryTokenAccount);
    let userStakingAccount = await program.account.userStakingAccount.fetch(
      userStakingPubkey
    );

    assert.equal(newBalance - oldBalance, userAuryRewardAmount.toNumber());
    assert.equal(userStakingAccount.claimableAuryAmount.toNumber(), 0);
  });

  it('Unstake success after claim', async () => {
    // Remaining accounts - mint(writable), vault(writable)
    let remainingAccounts = [
      {
        pubkey: userNFTTokenAccount[0],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[0],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[1],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[1],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[1],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[1],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[2],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[2],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[3],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[3],
        isWritable: true,
        isSigner: false,
      },
    ];

    await program.rpc.unstake(stakingBump, userStakingIndex, userStakingBump, {
      accounts: {
        nftToAuthority: provider.wallet.publicKey,
        stakingAccount: stakingPubkey,
        userStakingAccount: userStakingPubkey,
        tokenProgram: TOKEN_PROGRAM_ID,
      },
      remainingAccounts,
    });

    for (let i = 0; i < 4; i++) {
      assert.equal(await getTokenBalance(userNFTTokenAccount[i]), i + 1);

      await assert.rejects(
        async () => {
          await nftToken[i].getAccountInfo(nftVaultPubkey[i]);
        },
        {
          message: 'Failed to find account',
        }
      );
    }

    const userStakingAccount = await program.account.userStakingAccount.fetch(
      userStakingPubkey
    );
    assert.equal(userStakingAccount.nftMintKeys.toString(), [].toString());
  });

  it('Next stake success with empty-authorized-name-starts', async () => {
    // Remove AuthorizedNameStarts
    await program.rpc.removeAuthorizedNameStarts(
      stakingBump,
      authorizedNameStarts,
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
    assert.equal(stakingAccount.authorizedNameStarts.toString(), [].toString());

    // Next UserStakingAccount pda
    [nextUserStakingPubkey, nextUserStakingBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [
          Buffer.from(
            anchor.utils.bytes.utf8.encode(
              new anchor.BN(nextUserStakingIndex).toString()
            )
          ),
          provider.wallet.publicKey.toBuffer(),
        ],
        program.programId
      );

    // nftVaultBumps
    let nftVaultBumps = Buffer.from([nftVaultBump[0]]);

    // Remaining accounts - mint(readonly), metadata(readonly), tokenAccount(writable), vault(writable)
    let remainingAccounts = [
      {
        pubkey: nftMintPubkey[0],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: nftMetadataPubkey[0],
        isWritable: false,
        isSigner: false,
      },
      {
        pubkey: userNFTTokenAccount[0],
        isWritable: true,
        isSigner: false,
      },
      {
        pubkey: nftVaultPubkey[0],
        isWritable: true,
        isSigner: false,
      },
    ];

    await program.rpc.stake(
      nftVaultBumps,
      stakingBump,
      userStakingCounterBump,
      nextUserStakingBump,
      {
        accounts: {
          nftFromAuthority: provider.wallet.publicKey,
          stakingAccount: stakingPubkey,
          userStakingCounterAccount: userStakingCounterPubkey,
          userStakingAccount: nextUserStakingPubkey,
          systemProgram: anchor.web3.SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        },
        remainingAccounts,
      }
    );

    // nft balance of user and program
    assert.equal(await getTokenBalance(userNFTTokenAccount[0]), 0);
    assert.equal(await getTokenBalance(nftVaultPubkey[0]), 1);

    // next user staking account
    const nextUserStakingAccount =
      await program.account.userStakingAccount.fetch(nextUserStakingPubkey);
    assert.equal(
      nextUserStakingAccount.nftMintKeys.toString(),
      [nftMintPubkey[0]].toString()
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
