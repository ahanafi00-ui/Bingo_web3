# Frontend Integration Guide - BINGO Protocol

## 📋 Table of Contents
1. [Setup & Installation](#setup--installation)
2. [Contract Interaction Architecture](#contract-interaction-architecture)
3. [React Hooks & State Management](#react-hooks--state-management)
4. [Complete Code Examples](#complete-code-examples)
5. [Error Handling](#error-handling)
6. [UI/UX Best Practices](#uiux-best-practices)

---

## 1. Setup & Installation

### Install Dependencies

```bash
npm install @stellar/stellar-sdk
npm install @stellar/freighter-api
# or
npm install @soroban-react/core @soroban-react/freighter
```

### Environment Variables (.env)

```bash
# Network Configuration
NEXT_PUBLIC_NETWORK=testnet
NEXT_PUBLIC_RPC_URL=https://soroban-testnet.stellar.org

# Contract Addresses (after deployment)
NEXT_PUBLIC_BT_BILL_TOKEN_ADDRESS=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
NEXT_PUBLIC_BINGO_VAULT_ADDRESS=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
NEXT_PUBLIC_REPO_MARKET_ADDRESS=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
NEXT_PUBLIC_USDC_ADDRESS=CXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX

# Treasury Address (for admin operations)
NEXT_PUBLIC_TREASURY_ADDRESS=GXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
```

---

## 2. Contract Interaction Architecture

### 2.1 Contract Client Setup

```typescript
// lib/stellar/contracts.ts
import * as StellarSdk from '@stellar/stellar-sdk';
import { Contract, SorobanRpc } from '@stellar/stellar-sdk';

export const server = new SorobanRpc.Server(
  process.env.NEXT_PUBLIC_RPC_URL!
);

export const contracts = {
  btBillToken: new Contract(process.env.NEXT_PUBLIC_BT_BILL_TOKEN_ADDRESS!),
  bingoVault: new Contract(process.env.NEXT_PUBLIC_BINGO_VAULT_ADDRESS!),
  repoMarket: new Contract(process.env.NEXT_PUBLIC_REPO_MARKET_ADDRESS!),
  usdc: new Contract(process.env.NEXT_PUBLIC_USDC_ADDRESS!),
};

// Network passphrase
export const networkPassphrase = StellarSdk.Networks.TESTNET;
```

### 2.2 Wallet Connection

```typescript
// lib/stellar/wallet.ts
import freighter from '@stellar/freighter-api';

export interface WalletState {
  address: string | null;
  isConnected: boolean;
  publicKey: string | null;
}

export const connectWallet = async (): Promise<string> => {
  try {
    const isAllowed = await freighter.isAllowed();
    if (!isAllowed) {
      await freighter.setAllowed();
    }
    
    const publicKey = await freighter.requestAccess();
    return publicKey;
  } catch (error) {
    console.error('Failed to connect wallet:', error);
    throw new Error('Failed to connect wallet');
  }
};

export const disconnectWallet = () => {
  // Freighter doesn't have explicit disconnect
  // Just clear local state
};

export const signTransaction = async (
  xdr: string,
  publicKey: string
): Promise<string> => {
  try {
    const signedXDR = await freighter.signTransaction(xdr, {
      network: 'testnet',
      accountToSign: publicKey,
    });
    return signedXDR;
  } catch (error) {
    console.error('Failed to sign transaction:', error);
    throw error;
  }
};
```

---

## 3. React Hooks & State Management

### 3.1 Wallet Hook

```typescript
// hooks/useWallet.ts
import { useState, useEffect } from 'react';
import { connectWallet, WalletState } from '@/lib/stellar/wallet';

export const useWallet = () => {
  const [wallet, setWallet] = useState<WalletState>({
    address: null,
    isConnected: false,
    publicKey: null,
  });
  const [loading, setLoading] = useState(false);

  const connect = async () => {
    setLoading(true);
    try {
      const publicKey = await connectWallet();
      setWallet({
        address: publicKey,
        isConnected: true,
        publicKey,
      });
    } catch (error) {
      console.error('Wallet connection failed:', error);
    } finally {
      setLoading(false);
    }
  };

  const disconnect = () => {
    setWallet({
      address: null,
      isConnected: false,
      publicKey: null,
    });
  };

  return { wallet, connect, disconnect, loading };
};
```

### 3.2 Series Data Hook

```typescript
// hooks/useSeries.ts
import { useState, useEffect } from 'react';
import { getSeries, getCurrentPrice } from '@/lib/stellar/vault';

export interface Series {
  seriesId: number;
  issueDate: number;
  maturityDate: number;
  parUnit: bigint;
  issuePrice: bigint;
  capPar: bigint;
  mintedPar: bigint;
  userCapPar: bigint;
  status: 'Upcoming' | 'Active' | 'Matured' | 'Closed';
}

export const useSeries = (seriesId: number) => {
  const [series, setSeries] = useState<Series | null>(null);
  const [currentPrice, setCurrentPrice] = useState<bigint | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchData = async () => {
      try {
        setLoading(true);
        const [seriesData, price] = await Promise.all([
          getSeries(seriesId),
          getCurrentPrice(seriesId),
        ]);
        setSeries(seriesData);
        setCurrentPrice(price);
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to fetch series');
      } finally {
        setLoading(false);
      }
    };

    fetchData();
    
    // Refresh every 30 seconds
    const interval = setInterval(fetchData, 30000);
    return () => clearInterval(interval);
  }, [seriesId]);

  return { series, currentPrice, loading, error };
};
```

---

## 4. Complete Code Examples

### 4.1 BINGO VAULT - Subscribe (Buy T-Bills)

```typescript
// lib/stellar/vault.ts
import * as StellarSdk from '@stellar/stellar-sdk';
import { contracts, server, networkPassphrase } from './contracts';
import { signTransaction } from './wallet';

const SCALE = 10_000_000n; // 7 decimals

export const subscribe = async (
  userPublicKey: string,
  seriesId: number,
  payAmount: bigint // in USDC base units (7 decimals)
): Promise<string> => {
  try {
    // 1. Load user account
    const account = await server.getAccount(userPublicKey);
    
    // 2. Build transaction
    const transaction = new StellarSdk.TransactionBuilder(account, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase,
    })
      .addOperation(
        contracts.bingoVault.call(
          'subscribe',
          StellarSdk.Address.fromString(userPublicKey).toScVal(),
          StellarSdk.nativeToScVal(seriesId, { type: 'u32' }),
          StellarSdk.nativeToScVal(payAmount, { type: 'i128' })
        )
      )
      .setTimeout(180)
      .build();

    // 3. Simulate to get auth and resource fees
    const simulated = await server.simulateTransaction(transaction);
    
    if (StellarSdk.SorobanRpc.Api.isSimulationError(simulated)) {
      throw new Error(`Simulation failed: ${simulated.error}`);
    }

    // 4. Prepare transaction with simulation results
    const prepared = StellarSdk.SorobanRpc.assembleTransaction(
      transaction,
      simulated
    );

    // 5. Sign transaction
    const signedXDR = await signTransaction(
      prepared.toXDR(),
      userPublicKey
    );
    
    const signedTx = StellarSdk.TransactionBuilder.fromXDR(
      signedXDR,
      networkPassphrase
    );

    // 6. Submit transaction
    const result = await server.sendTransaction(signedTx);

    // 7. Wait for confirmation
    let status = await server.getTransaction(result.hash);
    while (status.status === 'PENDING' || status.status === 'NOT_FOUND') {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      status = await server.getTransaction(result.hash);
    }

    if (status.status === 'SUCCESS') {
      return result.hash;
    } else {
      throw new Error(`Transaction failed: ${status.status}`);
    }
  } catch (error) {
    console.error('Subscribe failed:', error);
    throw error;
  }
};

// Helper: Get Series Info
export const getSeries = async (seriesId: number) => {
  const account = await server.getAccount(
    'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF' // Dummy for read-only
  );

  const transaction = new StellarSdk.TransactionBuilder(account, {
    fee: '0',
    networkPassphrase,
  })
    .addOperation(
      contracts.bingoVault.call(
        'get_series',
        StellarSdk.nativeToScVal(seriesId, { type: 'u32' })
      )
    )
    .setTimeout(0)
    .build();

  const simulated = await server.simulateTransaction(transaction);
  
  if (StellarSdk.SorobanRpc.Api.isSimulationSuccess(simulated)) {
    return StellarSdk.scValToNative(simulated.result!.retval);
  }
  
  throw new Error('Failed to get series');
};

// Helper: Get Current Price
export const getCurrentPrice = async (seriesId: number): Promise<bigint> => {
  const account = await server.getAccount(
    'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF'
  );

  const transaction = new StellarSdk.TransactionBuilder(account, {
    fee: '0',
    networkPassphrase,
  })
    .addOperation(
      contracts.bingoVault.call(
        'current_price',
        StellarSdk.nativeToScVal(seriesId, { type: 'u32' })
      )
    )
    .setTimeout(0)
    .build();

  const simulated = await server.simulateTransaction(transaction);
  
  if (StellarSdk.SorobanRpc.Api.isSimulationSuccess(simulated)) {
    return StellarSdk.scValToNative(simulated.result!.retval);
  }
  
  throw new Error('Failed to get price');
};

// Helper: Format price from contract (7 decimals) to human readable
export const formatPrice = (price: bigint): string => {
  return (Number(price) / Number(SCALE)).toFixed(4);
};

// Helper: Calculate how much PAR user will get
export const calculateMintedPar = (
  payAmount: bigint,
  currentPrice: bigint
): bigint => {
  return (payAmount * SCALE) / currentPrice;
};
```

### 4.2 React Component - Subscribe Form

```typescript
// components/SubscribeForm.tsx
import { useState } from 'react';
import { useWallet } from '@/hooks/useWallet';
import { useSeries } from '@/hooks/useSeries';
import { subscribe, formatPrice, calculateMintedPar } from '@/lib/stellar/vault';

const SCALE = 10_000_000n;

export const SubscribeForm = ({ seriesId }: { seriesId: number }) => {
  const { wallet } = useWallet();
  const { series, currentPrice, loading: seriesLoading } = useSeries(seriesId);
  
  const [amount, setAmount] = useState('');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [txHash, setTxHash] = useState<string | null>(null);

  const handleSubscribe = async () => {
    if (!wallet.publicKey || !currentPrice) return;
    
    try {
      setLoading(true);
      setError(null);
      
      // Convert USDC amount to base units (7 decimals)
      const payAmount = BigInt(Math.floor(parseFloat(amount) * Number(SCALE)));
      
      const hash = await subscribe(wallet.publicKey, seriesId, payAmount);
      setTxHash(hash);
      setAmount('');
    } catch (err: any) {
      // Parse error code from Soroban
      const errorMatch = err.message.match(/Error\(Contract, #(\d+)\)/);
      if (errorMatch) {
        const errorCode = parseInt(errorMatch[1]);
        setError(getErrorMessage(errorCode));
      } else {
        setError(err.message || 'Transaction failed');
      }
    } finally {
      setLoading(false);
    }
  };

  const getErrorMessage = (code: number): string => {
    const errors: Record<number, string> = {
      20: 'Series not found',
      22: 'Series not active yet. Wait for treasury to activate.',
      30: 'Series is full. Try a smaller amount or wait for new series.',
      31: 'You have reached your personal cap for this series.',
      40: 'Invalid amount. Must be greater than zero.',
    };
    return errors[code] || `Transaction failed with error code ${code}`;
  };

  if (seriesLoading) return <div>Loading series...</div>;
  if (!series) return <div>Series not found</div>;

  const mintedPar = currentPrice && amount
    ? calculateMintedPar(
        BigInt(Math.floor(parseFloat(amount) * Number(SCALE))),
        currentPrice
      )
    : 0n;

  const available = series.capPar - series.mintedPar;

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-bold mb-4">Subscribe to Series {seriesId}</h2>
      
      {/* Series Info */}
      <div className="bg-gray-50 rounded p-4 mb-6 space-y-2">
        <div className="flex justify-between">
          <span className="text-gray-600">Current Price:</span>
          <span className="font-semibold">
            ${currentPrice ? formatPrice(currentPrice) : '...'}
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-600">Available:</span>
          <span className="font-semibold">
            {formatPrice(available)} PAR
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-600">Your Max:</span>
          <span className="font-semibold">
            {formatPrice(series.userCapPar)} PAR
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-600">Status:</span>
          <span className={`font-semibold ${
            series.status === 'Active' ? 'text-green-600' : 'text-gray-600'
          }`}>
            {series.status}
          </span>
        </div>
      </div>

      {/* Subscribe Form */}
      {series.status === 'Active' ? (
        <div className="space-y-4">
          <div>
            <label className="block text-sm font-medium text-gray-700 mb-2">
              Amount (USDC)
            </label>
            <input
              type="number"
              step="0.01"
              value={amount}
              onChange={(e) => setAmount(e.target.value)}
              className="w-full px-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-blue-500"
              placeholder="0.00"
              disabled={loading}
            />
          </div>

          {amount && mintedPar > 0n && (
            <div className="bg-blue-50 rounded p-4">
              <div className="text-sm text-gray-600">You will receive:</div>
              <div className="text-2xl font-bold text-blue-600">
                {formatPrice(mintedPar)} bT-Bills
              </div>
            </div>
          )}

          <button
            onClick={handleSubscribe}
            disabled={!amount || loading || !wallet.isConnected}
            className="w-full bg-blue-600 text-white py-3 rounded-lg font-semibold hover:bg-blue-700 disabled:bg-gray-300 disabled:cursor-not-allowed transition"
          >
            {loading ? 'Processing...' : 'Subscribe'}
          </button>

          {error && (
            <div className="bg-red-50 border border-red-200 rounded p-4 text-red-700">
              {error}
            </div>
          )}

          {txHash && (
            <div className="bg-green-50 border border-green-200 rounded p-4">
              <div className="text-sm text-gray-600">Transaction successful!</div>
              <a
                href={`https://stellar.expert/explorer/testnet/tx/${txHash}`}
                target="_blank"
                rel="noopener noreferrer"
                className="text-blue-600 hover:underline text-sm break-all"
              >
                View on Explorer →
              </a>
            </div>
          )}
        </div>
      ) : (
        <div className="bg-yellow-50 border border-yellow-200 rounded p-4 text-yellow-700">
          Series is {series.status}. 
          {series.status === 'Upcoming' && ' Wait for treasury to activate.'}
          {series.status === 'Matured' && ' You can now redeem your bT-Bills.'}
        </div>
      )}
    </div>
  );
};
```

### 4.3 REPO MARKET - Open Repo

```typescript
// lib/stellar/repo.ts
import * as StellarSdk from '@stellar/stellar-sdk';
import { contracts, server, networkPassphrase } from './contracts';
import { signTransaction } from './wallet';

export const openRepo = async (
  borrowerPublicKey: string,
  seriesId: number,
  collateralPar: bigint,
  desiredCashOut: bigint,
  deadline: number // Unix timestamp
): Promise<string> => {
  try {
    const account = await server.getAccount(borrowerPublicKey);
    
    const transaction = new StellarSdk.TransactionBuilder(account, {
      fee: StellarSdk.BASE_FEE,
      networkPassphrase,
    })
      .addOperation(
        contracts.repoMarket.call(
          'open_repo',
          StellarSdk.Address.fromString(borrowerPublicKey).toScVal(),
          StellarSdk.nativeToScVal(seriesId, { type: 'u32' }),
          StellarSdk.nativeToScVal(collateralPar, { type: 'i128' }),
          StellarSdk.nativeToScVal(desiredCashOut, { type: 'i128' }),
          StellarSdk.nativeToScVal(deadline, { type: 'u64' })
        )
      )
      .setTimeout(180)
      .build();

    const simulated = await server.simulateTransaction(transaction);
    
    if (StellarSdk.SorobanRpc.Api.isSimulationError(simulated)) {
      throw new Error(`Simulation failed: ${simulated.error}`);
    }

    const prepared = StellarSdk.SorobanRpc.assembleTransaction(
      transaction,
      simulated
    );

    const signedXDR = await signTransaction(
      prepared.toXDR(),
      borrowerPublicKey
    );
    
    const signedTx = StellarSdk.TransactionBuilder.fromXDR(
      signedXDR,
      networkPassphrase
    );

    const result = await server.sendTransaction(signedTx);

    let status = await server.getTransaction(result.hash);
    while (status.status === 'PENDING' || status.status === 'NOT_FOUND') {
      await new Promise((resolve) => setTimeout(resolve, 1000));
      status = await server.getTransaction(result.hash);
    }

    if (status.status === 'SUCCESS') {
      // Extract position ID from result
      return result.hash;
    } else {
      throw new Error(`Transaction failed: ${status.status}`);
    }
  } catch (error) {
    console.error('Open repo failed:', error);
    throw error;
  }
};

// Get repo position details
export const getRepoPosition = async (positionId: number) => {
  const account = await server.getAccount(
    'GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF'
  );

  const transaction = new StellarSdk.TransactionBuilder(account, {
    fee: '0',
    networkPassphrase,
  })
    .addOperation(
      contracts.repoMarket.call(
        'get_position',
        StellarSdk.nativeToScVal(positionId, { type: 'u64' })
      )
    )
    .setTimeout(0)
    .build();

  const simulated = await server.simulateTransaction(transaction);
  
  if (StellarSdk.SorobanRpc.Api.isSimulationSuccess(simulated)) {
    return StellarSdk.scValToNative(simulated.result!.retval);
  }
  
  throw new Error('Failed to get position');
};
```

### 4.4 React Component - Repo Dashboard

```typescript
// components/RepoDashboard.tsx
import { useState, useEffect } from 'react';
import { useWallet } from '@/hooks/useWallet';
import { getRepoPosition } from '@/lib/stellar/repo';
import { formatPrice } from '@/lib/stellar/vault';

interface RepoPosition {
  id: bigint;
  borrower: string;
  seriesId: number;
  collateralPar: bigint;
  cashOut: bigint;
  repurchaseAmount: bigint;
  startTime: bigint;
  deadline: bigint;
  status: 'Open' | 'Closed' | 'Defaulted';
}

export const RepoDashboard = () => {
  const { wallet } = useWallet();
  const [positions, setPositions] = useState<RepoPosition[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    // Fetch user's positions
    // In production, you'd query an indexer or maintain a position ID list
    const fetchPositions = async () => {
      try {
        // Example: fetch positions 1-10
        const positionsData = await Promise.allSettled(
          Array.from({ length: 10 }, (_, i) => getRepoPosition(i + 1))
        );
        
        const validPositions = positionsData
          .filter((p) => p.status === 'fulfilled')
          .map((p: any) => p.value)
          .filter((p) => p.borrower === wallet.address);
        
        setPositions(validPositions);
      } catch (error) {
        console.error('Failed to fetch positions:', error);
      } finally {
        setLoading(false);
      }
    };

    if (wallet.address) {
      fetchPositions();
    }
  }, [wallet.address]);

  const now = Math.floor(Date.now() / 1000);

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h2 className="text-2xl font-bold mb-6">My Repo Positions</h2>
      
      {loading ? (
        <div>Loading positions...</div>
      ) : positions.length === 0 ? (
        <div className="text-gray-500 text-center py-8">
          No repo positions found
        </div>
      ) : (
        <div className="space-y-4">
          {positions.map((position) => {
            const isOverdue = Number(position.deadline) < now;
            const daysUntilDeadline = Math.floor(
              (Number(position.deadline) - now) / 86400
            );

            return (
              <div
                key={Number(position.id)}
                className="border border-gray-200 rounded-lg p-4"
              >
                <div className="flex justify-between items-start mb-4">
                  <div>
                    <div className="text-sm text-gray-500">
                      Position #{Number(position.id)}
                    </div>
                    <div className="text-lg font-semibold">
                      Series {position.seriesId}
                    </div>
                  </div>
                  <span
                    className={`px-3 py-1 rounded-full text-sm font-medium ${
                      position.status === 'Open'
                        ? isOverdue
                          ? 'bg-red-100 text-red-700'
                          : 'bg-green-100 text-green-700'
                        : position.status === 'Closed'
                        ? 'bg-blue-100 text-blue-700'
                        : 'bg-gray-100 text-gray-700'
                    }`}
                  >
                    {position.status}
                    {position.status === 'Open' && isOverdue && ' (Overdue)'}
                  </span>
                </div>

                <div className="grid grid-cols-2 gap-4 text-sm">
                  <div>
                    <div className="text-gray-500">Collateral</div>
                    <div className="font-semibold">
                      {formatPrice(position.collateralPar)} bT-Bills
                    </div>
                  </div>
                  <div>
                    <div className="text-gray-500">Borrowed</div>
                    <div className="font-semibold">
                      ${formatPrice(position.cashOut)}
                    </div>
                  </div>
                  <div>
                    <div className="text-gray-500">Repayment</div>
                    <div className="font-semibold">
                      ${formatPrice(position.repurchaseAmount)}
                    </div>
                  </div>
                  <div>
                    <div className="text-gray-500">Deadline</div>
                    <div
                      className={`font-semibold ${
                        isOverdue ? 'text-red-600' : ''
                      }`}
                    >
                      {position.status === 'Open'
                        ? isOverdue
                          ? 'OVERDUE'
                          : `${daysUntilDeadline} days left`
                        : new Date(
                            Number(position.deadline) * 1000
                          ).toLocaleDateString()}
                    </div>
                  </div>
                </div>

                {position.status === 'Open' && !isOverdue && (
                  <button
                    onClick={() => {
                      /* Call close_repo */
                    }}
                    className="mt-4 w-full bg-blue-600 text-white py-2 rounded-lg font-semibold hover:bg-blue-700"
                  >
                    Repay & Close
                  </button>
                )}
              </div>
            );
          })}
        </div>
      )}
    </div>
  );
};
```

---

## 5. Error Handling

### Error Code Mapping

```typescript
// lib/stellar/errors.ts
export const BINGO_VAULT_ERRORS: Record<number, string> = {
  1: 'Contract already initialized',
  2: 'Contract not initialized',
  10: 'Unauthorized: You are not the admin or treasury',
  20: 'Series not found',
  21: 'Series ID already exists',
  22: 'Series not active yet. Wait for treasury to activate.',
  23: 'Series has not matured yet. You cannot redeem.',
  24: 'Invalid series status transition',
  30: 'Series is full. Try a smaller amount or wait for new series.',
  31: 'You have reached your personal cap for this series.',
  40: 'Invalid amount. Must be greater than zero.',
  41: 'Insufficient bT-Bills balance',
  50: 'Invalid timestamp: Maturity must be after issue date',
  51: 'Invalid issue price: Must be between 0 and PAR',
  52: 'Invalid cap amounts: user_cap must be ≤ series_cap',
  60: 'Contract is paused',
};

export const REPO_MARKET_ERRORS: Record<number, string> = {
  1: 'Contract already initialized',
  2: 'Contract not initialized',
  10: 'Unauthorized: Only treasury can perform this action',
  20: 'Repo position not found',
  21: 'Invalid position status for this operation',
  30: 'Invalid amount. Must be greater than zero.',
  31: 'Requested cash exceeds maximum LTV (collateral × price × (1 - haircut))',
  40: 'Invalid deadline: Must be ≤ series maturity date',
  41: 'Cannot claim default: Deadline has not passed yet',
  42: 'Cannot close repo: Deadline has already passed (defaulted)',
  50: 'Contract is paused',
};

export const parseContractError = (
  error: any,
  contractType: 'vault' | 'repo'
): string => {
  const errorMatch = error.message?.match(/Error\(Contract, #(\d+)\)/);
  
  if (errorMatch) {
    const errorCode = parseInt(errorMatch[1]);
    const errors =
      contractType === 'vault' ? BINGO_VAULT_ERRORS : REPO_MARKET_ERRORS;
    return errors[errorCode] || `Transaction failed with error code ${errorCode}`;
  }
  
  return error.message || 'Transaction failed';
};
```

### User-Friendly Error Component

```typescript
// components/ErrorDisplay.tsx
import { AlertCircle, XCircle, Info } from 'lucide-react';

interface ErrorDisplayProps {
  error: string;
  onDismiss?: () => void;
}

export const ErrorDisplay = ({ error, onDismiss }: ErrorDisplayProps) => {
  const getSeverity = (error: string): 'error' | 'warning' | 'info' => {
    if (error.includes('not active') || error.includes('not matured')) {
      return 'info';
    }
    if (error.includes('cap') || error.includes('full')) {
      return 'warning';
    }
    return 'error';
  };

  const severity = getSeverity(error);
  
  const styles = {
    error: 'bg-red-50 border-red-200 text-red-700',
    warning: 'bg-yellow-50 border-yellow-200 text-yellow-700',
    info: 'bg-blue-50 border-blue-200 text-blue-700',
  };

  const Icon = {
    error: XCircle,
    warning: AlertCircle,
    info: Info,
  }[severity];

  return (
    <div className={`border rounded-lg p-4 ${styles[severity]}`}>
      <div className="flex items-start gap-3">
        <Icon className="w-5 h-5 mt-0.5 flex-shrink-0" />
        <div className="flex-1">
          <p className="font-medium">{error}</p>
        </div>
        {onDismiss && (
          <button
            onClick={onDismiss}
            className="text-gray-400 hover:text-gray-600"
          >
            ×
          </button>
        )}
      </div>
    </div>
  );
};
```

---

## 6. UI/UX Best Practices

### 6.1 Loading States

```typescript
// components/LoadingSpinner.tsx
export const LoadingSpinner = ({ text = 'Loading...' }) => (
  <div className="flex flex-col items-center justify-center py-8">
    <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-600 mb-4" />
    <p className="text-gray-600">{text}</p>
  </div>
);

// Use in components
{loading && <LoadingSpinner text="Submitting transaction..." />}
```

### 6.2 Transaction Progress

```typescript
// components/TransactionProgress.tsx
export const TransactionProgress = ({ step }: { step: number }) => {
  const steps = [
    'Building transaction',
    'Simulating',
    'Signing with wallet',
    'Submitting to network',
    'Waiting for confirmation',
  ];

  return (
    <div className="space-y-2">
      {steps.map((label, index) => (
        <div key={index} className="flex items-center gap-3">
          <div
            className={`w-6 h-6 rounded-full flex items-center justify-center text-sm ${
              index < step
                ? 'bg-green-500 text-white'
                : index === step
                ? 'bg-blue-500 text-white animate-pulse'
                : 'bg-gray-200 text-gray-400'
            }`}
          >
            {index < step ? '✓' : index + 1}
          </div>
          <span
            className={
              index <= step ? 'text-gray-900' : 'text-gray-400'
            }
          >
            {label}
          </span>
        </div>
      ))}
    </div>
  );
};
```

### 6.3 Price Chart Component

```typescript
// components/PriceChart.tsx
import { useMemo } from 'react';
import { Line } from 'react-chartjs-2';
import { formatPrice } from '@/lib/stellar/vault';

const SCALE = 10_000_000n;

export const PriceChart = ({
  issueDate,
  maturityDate,
  issuePrice,
}: {
  issueDate: number;
  maturityDate: number;
  issuePrice: bigint;
}) => {
  const chartData = useMemo(() => {
    const now = Math.floor(Date.now() / 1000);
    const duration = maturityDate - issueDate;
    const points = 50;

    const labels: string[] = [];
    const prices: number[] = [];

    for (let i = 0; i <= points; i++) {
      const time = issueDate + (duration * i) / points;
      const elapsed = time - issueDate;
      const progress = elapsed / duration;

      const price =
        Number(issuePrice) +
        (Number(SCALE) - Number(issuePrice)) * progress;

      labels.push(new Date(time * 1000).toLocaleDateString());
      prices.push(Number(price) / Number(SCALE));
    }

    return {
      labels,
      datasets: [
        {
          label: 'Price',
          data: prices,
          borderColor: 'rgb(59, 130, 246)',
          backgroundColor: 'rgba(59, 130, 246, 0.1)',
          fill: true,
        },
      ],
    };
  }, [issueDate, maturityDate, issuePrice]);

  return (
    <div className="bg-white rounded-lg shadow p-6">
      <h3 className="text-lg font-semibold mb-4">Price Accretion</h3>
      <Line
        data={chartData}
        options={{
          responsive: true,
          plugins: {
            legend: {
              display: false,
            },
          },
          scales: {
            y: {
              beginAtZero: false,
              min: 0.9,
              max: 1.0,
            },
          },
        }}
      />
    </div>
  );
};
```

---

## 7. Complete App Structure

```
frontend/
├── app/
│   ├── page.tsx                    # Home/Dashboard
│   ├── series/[id]/page.tsx        # Series detail page
│   └── repo/page.tsx               # Repo market
├── components/
│   ├── WalletConnect.tsx
│   ├── SubscribeForm.tsx
│   ├── RedeemForm.tsx
│   ├── RepoDashboard.tsx
│   ├── PriceChart.tsx
│   ├── ErrorDisplay.tsx
│   └── LoadingSpinner.tsx
├── hooks/
│   ├── useWallet.ts
│   ├── useSeries.ts
│   ├── useBalance.ts
│   └── useRepoPositions.ts
├── lib/
│   └── stellar/
│       ├── contracts.ts
│       ├── wallet.ts
│       ├── vault.ts
│       ├── repo.ts
│       └── errors.ts
└── .env.local
```

---

## Quick Start Checklist

- [ ] Install dependencies (`@stellar/stellar-sdk`, `@stellar/freighter-api`)
- [ ] Set up environment variables with contract addresses
- [ ] Implement wallet connection
- [ ] Create contract interaction functions
- [ ] Build React components for each flow
- [ ] Add error handling with user-friendly messages
- [ ] Test on Stellar testnet
- [ ] Deploy frontend to Vercel/Netlify

---

Siap build frontend yang keren bang! 🚀
