'use client';

import dynamic from 'next/dynamic';
import { useState } from 'react';
import { PublicKey, LAMPORTS_PER_SOL } from '@solana/web3.js';
import { useConnection, useWallet } from '@solana/wallet-adapter-react';
import '@solana/wallet-adapter-react-ui/styles.css';
import { useProgram } from '@/hooks/useProgram';
import { createBond, DEFAULT_MINT } from '@/lib/bonded';

const WalletMultiButton = dynamic(
  async () => (await import('@solana/wallet-adapter-react-ui')).WalletMultiButton,
  { ssr: false }
);

export default function Home() {
  const { connection } = useConnection();
  const { publicKey } = useWallet();
  const program = useProgram();

  const [mint, setMint] = useState(DEFAULT_MINT.toBase58());
  const [arbiter, setArbiter] = useState('');
  const [amount, setAmount] = useState('100');
  const [disputeWindow, setDisputeWindow] = useState('3600');
  const [msg, setMsg] = useState('');
  const [sig, setSig] = useState('');
  const [busy, setBusy] = useState(false);

  const onCreateBond = async () => {
    if (!program || !publicKey) return;
    setBusy(true);
    setMsg('Creating bond, please approve in Phantom...');
    setSig('');
    try {
      const { signature, bond, vault } = await createBond(program, {
        provider: publicKey,
        mint: new PublicKey(mint.trim()),
        arbiter: new PublicKey((arbiter || publicKey.toBase58()).trim()),
        disputeWindow: Number(disputeWindow),
        amountTokens: Number(amount),
      });
      setSig(signature);
      setMsg(
        `Bond created!\nBond PDA: ${bond.toBase58()}\nVault PDA: ${vault.toBase58()}`
      );
      const bal = await connection.getBalance(publicKey);
      console.log('SOL balance:', bal / LAMPORTS_PER_SOL);
    } catch (e: unknown) {
      setMsg(`Error: ${e instanceof Error ? e.message : String(e)}`);
    } finally {
      setBusy(false);
    }
  };

  const input =
    'w-full rounded-xl border border-white/15 bg-black/30 px-4 py-3 text-white outline-none focus:border-indigo-400';

  return (
    <main className="min-h-screen bg-gradient-to-br from-slate-950 via-indigo-950 to-slate-900 px-4 py-10 text-white">
      <div className="mx-auto flex max-w-xl flex-col gap-5">
        <header className="flex flex-col items-center gap-3 text-center">
          <h1 className="text-3xl font-extrabold">
            Bonded <span className="text-indigo-400">x402</span>
          </h1>
          <WalletMultiButton />
        </header>

        <section className="flex flex-col gap-3 rounded-2xl border border-white/10 bg-white/5 p-5">
          <label className="text-sm text-white/60">Token Mint</label>
          <input className={input} value={mint} onChange={(e) => setMint(e.target.value)} />

          <label className="text-sm text-white/60">Arbiter (wallet address)</label>
          <input
            className={input}
            placeholder="Leave empty to use your wallet"
            value={arbiter}
            onChange={(e) => setArbiter(e.target.value)}
          />

          <div className="grid grid-cols-2 gap-3">
            <div className="flex flex-col gap-2">
              <label className="text-sm text-white/60">Amount (tokens)</label>
              <input className={input} type="number" value={amount} onChange={(e) => setAmount(e.target.value)} />
            </div>
            <div className="flex flex-col gap-2">
              <label className="text-sm text-white/60">Dispute window (sec)</label>
              <input className={input} type="number" value={disputeWindow} onChange={(e) => setDisputeWindow(e.target.value)} />
            </div>
          </div>

          <button
            onClick={onCreateBond}
            disabled={!program || !publicKey || busy}
            className="mt-2 rounded-xl bg-gradient-to-r from-indigo-500 to-purple-600 px-4 py-3 font-semibold disabled:opacity-40"
          >
            {busy ? 'Processing...' : 'Create Bond'}
          </button>
        </section>

        {msg && (
          <section className="whitespace-pre-wrap break-all rounded-2xl border border-white/10 bg-white/5 p-4 text-sm">
            <p>{msg}</p>
            {sig && (
              <a
                className="mt-2 inline-block underline"
                href={`https://explorer.solana.com/tx/${sig}?cluster=devnet`}
                target="_blank"
                rel="noreferrer"
              >
                View on Explorer ↗
              </a>
            )}
          </section>
        )}
      </div>
    </main>
  );
}
