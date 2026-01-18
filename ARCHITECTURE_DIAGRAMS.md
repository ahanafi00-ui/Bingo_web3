# BINGO Protocol - System Architecture Diagrams

## 1. Contract Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                         BINGO Protocol                          │
│                     Stellar/Soroban Network                     │
└─────────────────────────────────────────────────────────────────┘

                    ┌─────────────────────┐
                    │                     │
                    │   bt_bill_token     │
                    │   (Token Contract)  │
                    │                     │
                    └──────────┬──────────┘
                               │
                               │ operator
                               │ allowlist
                  ┌────────────┴────────────┐
                  │                         │
                  ▼                         ▼
       ┌──────────────────┐      ┌──────────────────┐
       │                  │      │                  │
       │  bingo_vault     │◄────►│  repo_market     │
       │  (Core Protocol) │      │  (Lending)       │
       │                  │      │                  │
       └────────┬─────────┘      └────────┬─────────┘
                │                         │
                │ uses                    │ uses
                │                         │
                ▼                         ▼
       ┌─────────────────────────────────────┐
       │                                     │
       │    Stablecoin (USDC)                │
       │    Stellar Asset Contract           │
       │                                     │
       └─────────────────────────────────────┘
```

## 2. Series Lifecycle State Machine

```
     ┌──────────┐
     │  CREATE  │  Treasury creates series with parameters
     └────┬─────┘  (issue_date, maturity, issue_price, caps)
          │
          │ create_series()
          │
          ▼
    ┌──────────┐
    │ UPCOMING │  Series created but not yet active
    │          │  Waiting for activation
    └────┬─────┘
         │
         │ activate_series() (admin/treasury)
         │
         ▼
    ┌────────┐
    │ ACTIVE │◄──────┐  Users can subscribe
    │        │       │  Price accretes over time
    └────┬───┘       │
         │           │
         │           │ subscribe() (users)
         │           │ └─► mint bT-Bills
         │           │
         │ time ≥ maturity_date
         │
         ▼
    ┌─────────┐
    │ MATURED │  Users can redeem bT-Bills
    │         │  for PAR value (1:1)
    └────┬────┘
         │
         │ redeem() (users)
         │ └─► burn bT-Bills
         │
         ▼
    ┌─────────┐
    │ CLOSED  │  Series ended
    │         │  (optional final state)
    └─────────┘
```

## 3. Subscription Flow (Detailed)

```
┌─────────┐                                                          ┌────────────┐
│         │ 1. User wants to buy T-Bills                            │            │
│  USER   │ ───────────────────────────────────────────────────────►│   VAULT    │
│         │    subscribe(series_id: 1, pay_amount: 1000 USDC)      │            │
└─────────┘                                                          └──────┬─────┘
                                                                            │
                                                                            │ 2. Check series
                                                                            │    - Status = ACTIVE?
                                                                            │    - Calculate price
                                                                            │
                                                                            ▼
                                    Price Calculation:
                                    price(t) = 0.98 + (1.0 - 0.98) × elapsed/total
                                    minted_par = 1000 × 1.0 / 0.98 = 1020.41 PAR
                                                                            │
                                                                            │ 3. Validate caps
                                                                            │    ✓ minted_par ≤ cap
                                                                            │    ✓ user_par ≤ user_cap
                                                                            │
    ┌──────────────┐                                                       ▼
    │              │◄────── 4. Transfer 1000 USDC from user to vault ─────┤
    │  STABLECOIN  │
    │   (USDC)     │
    └──────────────┘
                                                                            │
    ┌──────────────┐                                                       │
    │              │◄────── 5. Mint 1020.41 PAR to user ───────────────────┤
    │ BT_BILL_     │        bt_bill_token.mint(series=1, user, 1020.41)    │
    │   TOKEN      │                                                        │
    └──────────────┘                                                       │
                                                                            │
                                                                            │ 6. Update state
                                                                            │    series.minted_par += 1020.41
                                                                            │    user.subscribed_par += 1020.41
                                                                            │
    ┌──────────────┐                                                       │
    │              │◄────── 7. Emit Subscribed event ──────────────────────┤
    │   EVENTS     │        (series_id, user, pay, minted, price)          │
    └──────────────┘                                                       └─────

Result: User has 1020.41 bT-Bills representing claim on 1020.41 USDC at maturity
```

## 4. Repo Market Flow (Open → Close)

```
HAPPY PATH: Borrower repays before deadline
════════════════════════════════════════════

┌──────────┐                ┌──────────┐              ┌───────────┐            ┌──────────┐
│ BORROWER │                │   REPO   │              │  BT_BILL  │            │ TREASURY │
│          │                │  MARKET  │              │   TOKEN   │            │          │
└────┬─────┘                └────┬─────┘              └─────┬─────┘            └────┬─────┘
     │                           │                          │                       │
     │ open_repo()               │                          │                       │
     │ series=1, collateral=10k  │                          │                       │
     │ cash_request=9.5k         │                          │                       │
     │ deadline=+30days          │                          │                       │
     ├──────────────────────────►│                          │                       │
     │                           │ Calculate max cash:      │                       │
     │                           │ value = 10k × 0.99 = 9.9k│                       │
     │                           │ max = 9.9k × 97% = 9.6k  │                       │
     │                           │ ✓ 9.5k < 9.6k (OK)       │                       │
     │                           │                          │                       │
     │                           │ Transfer collateral      │                       │
     │                           ├─────────────────────────►│                       │
     │                           │ transfer(borrower→repo, 10k PAR)                 │
     │                           │                          │                       │
     │                           │ Transfer cash            │                       │
     │◄──────────────────────────┼──────────────────────────┼───────────────────────┤
     │   9,500 USDC              │                          │  transfer(treasury→borrower)
     │                           │                          │                       │
     │   Position ID: 1          │ Store position:          │                       │
     │◄──────────────────────────┤ - collateral: 10k        │                       │
     │                           │ - cash_out: 9.5k         │                       │
     │                           │ - repurchase: 9.69k      │                       │
     │                           │   (9.5k × 102%)          │                       │
     │                           │ - status: OPEN           │                       │
     │                           │                          │                       │
     │       ⏰ Time passes       │                          │                       │
     │       (before deadline)   │                          │                       │
     │                           │                          │                       │
     │ close_repo(position_id=1) │                          │                       │
     ├──────────────────────────►│                          │                       │
     │                           │ Verify deadline OK       │                       │
     │                           │                          │                       │
     │                           │ Transfer repurchase amount                       │
     ├───────────────────────────┼──────────────────────────┼──────────────────────►│
     │   9,690 USDC              │                          │   transfer(borrower→treasury)
     │                           │                          │                       │
     │                           │ Return collateral        │                       │
     │◄──────────────────────────┼──────────────────────────┤                       │
     │   10,000 PAR              │  transfer(repo→borrower, 10k PAR)                │
     │                           │                          │                       │
     │                           │ Update: status=CLOSED    │                       │
     │                           │                          │                       │

Treasury profit: 190 USDC (2% spread)
Borrower cost: 190 USDC for 30-day liquidity
```

## 5. Repo Market Flow (Default)

```
DEFAULT PATH: Borrower fails to repay
═════════════════════════════════════

┌──────────┐                ┌──────────┐              ┌───────────┐            ┌──────────┐
│ BORROWER │                │   REPO   │              │  BT_BILL  │            │ TREASURY │
│          │                │  MARKET  │              │   TOKEN   │            │          │
└────┬─────┘                └────┬─────┘              └─────┬─────┘            └────┬─────┘
     │                           │                          │                       │
     │ [Position already open]   │                          │                       │
     │ collateral: 10k PAR       │                          │                       │
     │ borrowed: 9.5k USDC       │                          │                       │
     │ deadline: Day 30          │                          │                       │
     │                           │                          │                       │
     │       ⏰ Time passes       │                          │                       │
     │       deadline expires    │                          │                       │
     │       (Day 31)            │                          │                       │
     │                           │                          │                       │
     │   ❌ No repayment         │                          │                       │
     │                           │                          │                       │
     │                           │                          │                       │
     │                           │ claim_default(pos=1)     │                       │
     │                           │◄─────────────────────────┼───────────────────────┤
     │                           │                          │   (treasury calls)    │
     │                           │ Verify deadline passed   │                       │
     │                           │ ✓ current_time > deadline│                       │
     │                           │                          │                       │
     │                           │ Transfer collateral      │                       │
     │                           ├─────────────────────────►│                       │
     │                           │                          ├──────────────────────►│
     │                           │  transfer(repo→treasury, 10k PAR)                │
     │                           │                          │                       │
     │                           │ Update: status=DEFAULTED │                       │
     │                           │                          │                       │

Result:
- Borrower loses 10k PAR collateral (worth ~10k USDC at maturity)
- Treasury keeps both 10k PAR (can redeem for 10k USDC) AND the 9.5k USDC already lent
- Treasury profit: ~500 USDC (10k - 9.5k)
```

## 6. Price Accretion Timeline

```
Price Evolution Over 90-Day T-Bill
═══════════════════════════════════

Day    Price    Value/1000 PAR   Daily Yield
──────────────────────────────────────────────
0      0.9500   $950.00         Issue
10     0.9556   $955.56         +0.056%
20     0.9611   $961.11         +0.056%
30     0.9667   $966.67         +0.056%
40     0.9722   $972.22         +0.056%
50     0.9778   $977.78         +0.056%
60     0.9833   $983.33         +0.056%
70     0.9889   $988.89         +0.056%
80     0.9944   $994.44         +0.056%
90     1.0000   $1,000.00       Maturity

Total Return: 5.26% (50/950)
Daily Return: 0.0584% (5.26%/90)
Annualized: 21.32% (5.26% × 365/90)

Formula: price(t) = 0.95 + (1.0 - 0.95) × (t/90)

Visual:
1.00 ┤                                                              ●
     │                                                          ╱
0.99 ┤                                                      ╱
     │                                                  ╱
0.98 ┤                                              ╱
     │                                          ╱
0.97 ┤                                      ╱
     │                                  ╱
0.96 ┤                              ╱
     │                          ╱
0.95 ┤●                     ╱
     └────────────────────────────────────────────────────────────►
     0    10   20   30   40   50   60   70   80   90  Days
```

## 7. System Invariants

```
Critical Invariants (MUST hold at all times)
═════════════════════════════════════════════

1. Token Supply
   ∀ series: Σ(user_balances) = series.minted_par
   
2. Series Cap
   ∀ series: series.minted_par ≤ series.cap_par
   
3. User Cap
   ∀ (series, user): user.subscribed_par ≤ series.user_cap_par
   
4. Price Bounds
   ∀ series: issue_price ≤ current_price(t) ≤ PAR_UNIT
   
5. Repo Collateral
   ∀ open_position: collateral locked in escrow
   
6. Repo LTV
   ∀ position: cash_out ≤ collateral_value × (1 - haircut)
   
7. Redemption
   Only when: current_time ≥ maturity_date
   
8. Authorization
   mint/burn: only authorized operators
   admin ops: only admin/treasury
   user ops: requires user signature
```

## 8. Data Flow Summary

```
┌─────────────────────────────────────────────────────────────┐
│                     OFF-CHAIN                               │
│                                                             │
│  1. Treasury buys T-Bills ($100M)                          │
│  2. Settles with broker/dealer                             │
│  3. Holds physical T-Bills in custody                      │
│                                                             │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       │ 3. Create on-chain Series
                       │    cap = $100M
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                     ON-CHAIN                                │
│                                                             │
│  4. Series ACTIVE                                           │
│  5. Users subscribe → mint bT-Bills                         │
│  6. Price accretes daily                                    │
│  7. Optional: Users open repos for liquidity                │
│  8. At maturity: Users redeem → burn bT-Bills              │
│  9. Treasury pays out from T-Bill proceeds                  │
│                                                             │
└─────────────────────────────────────────────────────────────┘
                       │
                       │ 10. Off-chain settlement
                       ▼
┌─────────────────────────────────────────────────────────────┐
│                     OFF-CHAIN                               │
│                                                             │
│  11. Treasury redeems T-Bills from US Gov                  │
│  12. Receives USD                                           │
│  13. Converts to USDC (if needed)                          │
│  14. On-chain vault has USDC to honor redemptions          │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

**Legend:**
- → : Action/Flow
- ◄─ : Response/Return
- ├─ : Branch/Fork
- ▼ : State Transition
- ● : Data Point
- ═ : Section Separator
- ✓ : Validation Success
- ❌ : Failure/Error
