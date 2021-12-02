import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { TOKEN_PROGRAM_ID, Token } from '@solana/spl-token';
import assert from 'assert';
import { NftStaking } from '../target/types/nft_staking';
import { createMint } from './utils/upload_nft';

let program = anchor.workspace.NftStaking as Program<NftStaking>;
const envProvider = anchor.Provider.env();
let provider = envProvider;

function setProvider(p: anchor.Provider) {
  provider = p;
  anchor.setProvider(p);
  program = new anchor.Program(
    program.idl,
    program.programId,
    p
  ) as Program<NftStaking>;
}
setProvider(provider);

describe('nft-staking', () => {
  //the program's account for stored initializer key
  let stakingPubkey;
  let stakingBump;

  it('Mint NFT', async () => {
    const data = {
      name: 'Helios 3D',
      symbol: '',
      description: 'Helios artwork for Aurorians and some $AURY holders',
      seller_fee_basis_points: 1000,
      image:
        'https://arweave.net/ZPoeB7tMyKfAc_vrNfTfb7XCKhoccW8DS3UgUyacQMU?ext=gif',
      animation_url:
        'https://arweave.net/ZXSDhLbbD2gOn2Ch95hgCpvOZoC5RQntKFOuzC-yAqU?ext=mp4',
      external_url: '',
      collection: { name: 'Helios 3D', family: 'Aurory' },
      properties: {
        files: [
          {
            uri: 'https://arweave.net/ZPoeB7tMyKfAc_vrNfTfb7XCKhoccW8DS3UgUyacQMU?ext=gif',
            type: 'image/gif',
          },
          {
            uri: 'https://arweave.net/ZXSDhLbbD2gOn2Ch95hgCpvOZoC5RQntKFOuzC-yAqU?ext=mp4',
            type: 'video/mp4',
          },
        ],
        category: 'video',
        creators: [
          {
            address: '2j85gueUvAFeFEdKZE5yKAvyAsU8fKKZvxX8zLbX8GCc',
            share: 100,
          },
        ],
      },
    };
    const json_url = `https://arweave.net/uKoxW5gu2A7Wem-tgyWZ9-T46aAg49Gac-n0GNibTjI`;
    const lamports = await Token.getMinBalanceRentForExemptMint(
      provider.connection
    );
    const [keypair, tx] = await createMint(
      provider.wallet.publicKey,
      provider.wallet.publicKey,
      lamports,
      data,
      json_url
    );
    const signers = [keypair];

    await provider.send(tx, signers);
  });

  it('Is initialized!', async () => {
    [stakingPubkey, stakingBump] =
      await anchor.web3.PublicKey.findProgramAddress(
        [Buffer.from(anchor.utils.bytes.utf8.encode('nft_staking'))],
        program.programId
      );

    await program.rpc.initialize(stakingBump, {
      accounts: {
        initializer: provider.wallet.publicKey,
        stakingAccount: stakingPubkey,
        systemProgram: anchor.web3.SystemProgram.programId,
        tokenProgram: TOKEN_PROGRAM_ID,
        rent: anchor.web3.SYSVAR_RENT_PUBKEY,
      },
    });
  });
});
