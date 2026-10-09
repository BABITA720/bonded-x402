# Bonded x402

Trust-minimized paid services on Solana. A provider locks a bond, a buyer pays into escrow, and disputes and refunds are enforced by the program.

## The problem
Paid APIs and AI agents have no way to prove reliability before payment. Buyers risk losing funds, and providers have no stake in good service.

## The solution
- **Bond:** the provider deposits SPL tokens into a bond vault as collateral.
- **Escrow:** buyers pay into an escrow receipt tied to the bond.
- **Dispute and refund:** a buyer can dispute within a time window, and an arbiter resolves it on-chain.

## Live on Devnet
- Program ID: `CtXSSkVzvobPWuLZeciRcKa3fyXTUEgwmS1sUQnK9QZz`
- Example Create Bond transaction: [Solana Explorer](https://explorer.solana.com/tx/3m5U8MVbVS7TmPGCZTEbfdqW8eeQFTBrMVe6GUgPe3GKgd8BB2VT6rkr3AH1JFcutEbLcXeTkFSaQ7dcLWHpFSCk?cluster=devnet)

## Architecture
| Account | Seeds |
|---|---|
| Bond | `["bond", provider]` |
| Vault (token account) | `["vault", bond]` |
| Receipt | `["receipt", bond, buyer, nonce]` |

## Instructions
`create_bond`, `top_up`, `pay_and_access`, `dispute`, `release`, `resolve`, `refund`

## Run locally
```bash
anchor build
cd frontend && npm install && npm run dev
