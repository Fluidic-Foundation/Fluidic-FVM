export interface Token {
  symbol: string;
  name: string;
  decimals: number;
  color: string;
}

export const TOKENS: Token[] = [
  // The Rust mesh_node uses 12 fixed decimal places for both pool tokens.
  { symbol: 'WAVE', name: 'Fluidic Wave', decimals: 12, color: '#00E5C9' },
  { symbol: 'USDC', name: 'USD Coin', decimals: 12, color: '#3B82F6' },
  { symbol: 'ETH', name: 'Ether', decimals: 12, color: '#8B5CF6' },
];

export function formatAmount(value: number, decimals: number): string {
  return value.toLocaleString('en-US', {
    minimumFractionDigits: 0,
    maximumFractionDigits: decimals,
  });
}
