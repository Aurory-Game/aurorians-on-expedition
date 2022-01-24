import { programs } from '@metaplex/js';
import { Keypair, PublicKey, SystemProgram } from '@solana/web3.js';
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  MintLayout,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import { Token } from '@solana/spl-token';
import { TokenInstructions } from '@project-serum/serum';
import { web3, Provider } from '@project-serum/anchor';

const { Metadata, MetadataDataData, CreateMetadata, Creator } =
  programs.metadata;
const Transaction = programs.Transaction;

export function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export async function createMint(
  fee_payer: PublicKey,
  dest_owner: PublicKey,
  lamports,
  data: any,
  json_url: string
): Promise<[Keypair, PublicKey, programs.Transaction]> {
  const mint = Keypair.generate();
  console.log(`https://solscan.io/token/${mint.publicKey.toString()}`);
  const tx_mint = new Transaction({ feePayer: fee_payer });
  let ata = await Token.getAssociatedTokenAddress(
    ASSOCIATED_TOKEN_PROGRAM_ID, // always associated token program id
    TOKEN_PROGRAM_ID, // always token program id
    mint.publicKey, // mint
    dest_owner // token account authority,
  );
  tx_mint.add(
    // create mint
    SystemProgram.createAccount({
      fromPubkey: fee_payer,
      newAccountPubkey: mint.publicKey,
      space: MintLayout.span,
      lamports: lamports,
      programId: TOKEN_PROGRAM_ID,
    }),
    Token.createInitMintInstruction(
      TOKEN_PROGRAM_ID,
      mint.publicKey,
      0,
      fee_payer,
      fee_payer
    ),
    // create token account
    Token.createAssociatedTokenAccountInstruction(
      ASSOCIATED_TOKEN_PROGRAM_ID,
      TOKEN_PROGRAM_ID,
      mint.publicKey,
      ata,
      dest_owner,
      fee_payer
    ),
    // mint to token account
    Token.createMintToInstruction(
      TOKEN_PROGRAM_ID,
      mint.publicKey,
      ata,
      fee_payer,
      [],
      1
    )
  );

  const metadataPDA = await Metadata.getPDA(mint.publicKey);
  const metadataData = new MetadataDataData({
    name: data.name,
    symbol: '',
    uri: json_url,
    sellerFeeBasisPoints: data.seller_fee_basis_points,
    creators: [
      new Creator({
        address: fee_payer.toString(),
        verified: true,
        share: 100,
      }),
    ],
  });
  const tx_metadata = new CreateMetadata(
    {
      feePayer: fee_payer,
    },
    {
      metadata: metadataPDA,
      metadataData,
      updateAuthority: fee_payer,
      mint: mint.publicKey,
      mintAuthority: fee_payer,
    }
  );

  const tx = Transaction.fromCombined([tx_mint, tx_metadata]);
  return [mint, metadataPDA, tx];
}

export async function setMintAuthority(
  provider: Provider,
  mint: PublicKey,
  newAuthority: PublicKey
) {
  const tx = new Transaction();
  tx.add(
    Token.createSetAuthorityInstruction(
      TOKEN_PROGRAM_ID,
      mint,
      newAuthority,
      'MintTokens',
      provider.wallet.publicKey,
      []
    )
  );
  await provider.send(tx);
}

export async function mintToAccount(
  provider: Provider,
  mint: PublicKey,
  destination: PublicKey,
  amount: number
) {
  const tx = new Transaction();
  tx.add(
    Token.createMintToInstruction(
      TOKEN_PROGRAM_ID,
      mint,
      destination,
      provider.wallet.publicKey,
      [],
      amount
    )
  );
  await provider.send(tx);
}

export async function createTokenAccount(
  provider: Provider,
  mint: PublicKey,
  owner: PublicKey
) {
  const vault = Keypair.generate();
  const tx = new Transaction();
  tx.add(
    ...(await createTokenAccountInstrs(provider, vault.publicKey, mint, owner))
  );
  await provider.send(tx, [vault]);
  return vault.publicKey;
}

export async function createTokenAccountInstrs(
  provider: Provider,
  newAccountPubkey: PublicKey,
  mint: PublicKey,
  owner: PublicKey,
  lamports: number | undefined = undefined
) {
  if (lamports === undefined) {
    lamports = await provider.connection.getMinimumBalanceForRentExemption(165);
  }
  return [
    SystemProgram.createAccount({
      fromPubkey: provider.wallet.publicKey,
      newAccountPubkey,
      space: 165,
      lamports,
      programId: TOKEN_PROGRAM_ID,
    }),
    TokenInstructions.initializeAccount({
      account: newAccountPubkey,
      mint,
      owner,
    }),
  ];
}

export async function createTokenMint(
  provider: Provider,
  mintAccount: Keypair,
  mintAuthority: PublicKey,
  freezeAuthority: PublicKey | null,
  decimals: number,
  programId: PublicKey
) {
  const payer = web3.Keypair.generate();

  //airdrop tokens
  await provider.connection.confirmTransaction(
    await provider.connection.requestAirdrop(
      payer.publicKey,
      1 * web3.LAMPORTS_PER_SOL
    ),
    'confirmed'
  );

  const token = new Token(
    provider.connection,
    mintAccount.publicKey,
    programId,
    payer
  );

  // Allocate memory for the account
  const balanceNeeded = await Token.getMinBalanceRentForExemptMint(
    provider.connection
  );

  const transaction = new web3.Transaction();
  transaction.add(
    web3.SystemProgram.createAccount({
      fromPubkey: provider.wallet.publicKey,
      newAccountPubkey: mintAccount.publicKey,
      lamports: balanceNeeded,
      space: MintLayout.span,
      programId,
    })
  );

  transaction.add(
    Token.createInitMintInstruction(
      programId,
      mintAccount.publicKey,
      decimals,
      mintAuthority,
      freezeAuthority
    )
  );

  await provider.send(transaction, [mintAccount]);
  return token;
}
