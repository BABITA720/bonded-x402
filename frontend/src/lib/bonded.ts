import { PublicKey, SystemProgram } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import {
  getAssociatedTokenAddressSync,
  TOKEN_PROGRAM_ID,
} from '@solana/spl-token';

export const PROGRAM_ID = new PublicKey(
  'CtXSSkVzvobPWuLZeciRcKa3fyXTUEgwmS1sUQnK9QZz'
);
export const DEFAULT_MINT = new PublicKey(
  '9qYXY48RuqThNSP9dmqjKM9E5bYy7ihmQ2VVRQEFSPzc'
);
export const TOKEN_DECIMALS = 9;

const enc = (s: string) => new TextEncoder().encode(s);

// Seeds: bond = ["bond", provider], vault = ["vault", bond]
export const getBondPda = (provider: PublicKey) =>
  PublicKey.findProgramAddressSync(
    [enc('bond'), provider.toBuffer()],
    PROGRAM_ID
  )[0];

export const getVaultPda = (bond: PublicKey) =>
  PublicKey.findProgramAddressSync(
    [enc('vault'), bond.toBuffer()],
    PROGRAM_ID
  )[0];

// Convert whole tokens (e.g. 100) to raw units (100 * 10^9)
export const tokensToRaw = (tokens: number) =>
  new BN(Math.round(tokens * 10 ** TOKEN_DECIMALS).toString());

export interface CreateBondParams {
  provider: PublicKey;
  mint: PublicKey;
  arbiter: PublicKey;
  disputeWindow: number; // seconds
  amountTokens: number;
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function createBond(program: any, p: CreateBondParams) {
  const bond = getBondPda(p.provider);
  const vault = getVaultPda(bond);
  const providerAta = getAssociatedTokenAddressSync(
    p.mint,
    p.provider,
    false,
    TOKEN_PROGRAM_ID
  );

  const signature: string = await program.methods
    .createBond(p.arbiter, new BN(p.disputeWindow), tokensToRaw(p.amountTokens))
    .accounts({
      provider: p.provider,
      mint: p.mint,
      bond,
      vault,
      providerAta,
      tokenProgram: TOKEN_PROGRAM_ID,
      systemProgram: SystemProgram.programId,
    })
    .rpc();

  return { signature, bond, vault };
}
