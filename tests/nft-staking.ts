import * as anchor from '@project-serum/anchor';
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token';
import assert from 'assert';
import { nft_data, nft_json_url } from './data';
import { createMint, setMintAuthority } from './utils';
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

  it('Is initialized!', async () => {
    [stakingPubkey, stakingBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(anchor.utils.bytes.utf8.encode('nft_staking'))],
        program.programId
      );

    await program.rpc.initialize(stakingBump, provider.wallet.publicKey, {
      accounts: {
        stakingAccount: stakingPubkey,
        initializer: provider.wallet.publicKey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
    });
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
  });

  // it('Stake match NFT', async () => {
  //   await program.rpc.stake(stakingBump, {
  //     accounts: {
  //       tokenFromAuthority: provider.wallet.publicKey,
  //       tokenMetadata: metadataPubkey,
  //       stakingAccount: stakingPubkey,
  //     },
  //   });
  // });

  // it('Update initializer', async () => {
  //   await program.rpc.updateInitializer(stakingBump, {
  //     accounts: {
  //       initializer: provider.wallet.publicKey,
  //       newInitializer: '2j85gueUvAFeFEdKZE5yKAvyAsU8fKKZvxX8zLbX8GCc',
  //       stakingAccount: stakingPubkey,
  //     },
  //   });
  // });

  // it('Stake non-match NFT', async () => {
  //   await assert.rejects(
  //     async () => {
  //       await program.rpc.stake(stakingBump, {
  //         accounts: {
  //           tokenFromAuthority: provider.wallet.publicKey,
  //           tokenMetadata: metadataPubkey,
  //           stakingAccount: stakingPubkey,
  //         },
  //       });
  //     },
  //     { code: 300, msg: 'NoCreatorsFoundInMetadata' }
  //   );
  // });
});
