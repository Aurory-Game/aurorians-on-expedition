import * as anchor from '@project-serum/anchor';
import { Program } from '@project-serum/anchor';
import { NftStaking } from '../target/types/nft_staking';

describe('nft-staking', () => {

  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.Provider.env());

  const program = anchor.workspace.NftStaking as Program<NftStaking>;

  it('Is initialized!', async () => {
    // Add your test here.
    const tx = await program.rpc.initialize({});
    console.log("Your transaction signature", tx);
  });
});
