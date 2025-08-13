# Trading Bot Examples

This package contains comprehensive examples of trading bots built with the Nyks Wallet SDK for the Twilight Protocol. These examples demonstrate various trading strategies, risk management techniques, and automated execution patterns.

## 📑 Table of Contents

1. [Overview](#overview)
2. [Prerequisites](#prerequisites)
3. [Installation](#installation)
4. [Available Bots](#available-bots)
5. [Configuration](#configuration)
6. [Usage Examples](#usage-examples)
7. [Strategy Details](#strategy-details)
8. [Risk Management](#risk-management)
9. [Best Practices](#best-practices)
10. [Troubleshooting](#troubleshooting)

## Overview

The trading bot examples showcase how to:

- **Build sophisticated trading strategies** using the Nyks Wallet SDK
- **Manage ZkOS privacy-preserving accounts** for trading operations
- **Implement risk management** with position sizing and stop losses
- **Handle automated execution** with proper error handling and retry logic
- **Monitor performance** with comprehensive statistics and logging

All bots support both **live trading** and **paper trading** modes, making them perfect for testing strategies before risking real capital.

## Prerequisites

Before running the trading bots, ensure you have:

1. **Rust toolchain** (1.70 or later)
2. **Access to a Twilight Protocol testnet or mainnet**
3. **Funded wallet** with sufficient NYKS and BTC for trading
4. **Environment configuration** (see [Configuration](#configuration))

### System Requirements

- Linux/macOS/Windows with WSL
- At least 4GB RAM
- Stable internet connection
- Optional: Multiple cores for concurrent strategy execution

## Installation

1. **Clone the repository** (if not already done):

   ```bash
   cd examples/trading_bot
   ```

2. **Install dependencies**:

   ```bash
   cargo build --release
   ```

3. **Set up environment** (see [Configuration](#configuration)):

   ```bash
   cp .env.example .env
   # Edit .env with your configuration
   ```

4. **Verify installation**:
   ```bash
   cargo run --bin main -- --help
   ```

## Available Bots

### 1. Main Trading Bot (`main.rs`)

**Comprehensive multi-strategy trading bot**

- **Strategies**: Momentum, mean reversion, arbitrage, market making
- **Features**: Advanced risk management, multi-account setup, performance tracking
- **Best for**: Sophisticated traders, strategy research, portfolio diversification

```bash
cargo run --bin main -- --strategy momentum --initial-capital 100000 --max-leverage 10
```

### 2. Simple Market Maker (`simple_market_maker.rs`)

**Automated market making with spread capture**

- **Strategy**: Places buy/sell orders around market price
- **Features**: Inventory management, dynamic spreads, risk controls
- **Best for**: Liquidity provision, consistent returns, low-risk trading

```bash
# Basic market maker with real price data from relayer API
cargo run --bin simple_market_maker -- --spread 0.002 --order-size 1000 --max-inventory 10000

# Enhanced market maker with order book analysis and recent trades
cargo run --bin simple_market_maker -- --spread 0.002 --order-size 1000 --enhanced-market-data
```

### 3. Momentum Trader (`momentum_trader.rs`)

**Technical analysis-based trend following**

- **Strategy**: Moving averages, RSI, momentum indicators
- **Features**: Multiple timeframes, signal strength analysis, adaptive position sizing
- **Best for**: Trend following, technical analysis, medium-term positions

```bash
cargo run --bin momentum_trader -- --fast-ma 10 --slow-ma 30 --rsi-period 14 --position-size 5000
```

### 4. Lending Bot (`lending_bot.rs`)

**Automated liquidity provision for yield generation**

- **Strategy**: Provides liquidity to lending markets
- **Features**: Rate optimization, auto-reinvestment, exposure management
- **Best for**: Passive income, low-risk yield generation, capital efficiency

```bash
cargo run --bin lending_bot -- --min-rate 0.05 --max-exposure 0.8 --lending-amount 10000
```

### 5. Market Data Test (`test_market_data.rs`)

**API integration testing utility**

- **Purpose**: Tests relayer API market data endpoints
- **Features**: Price fetching, order book analysis, recent trades, funding rates
- **Best for**: API testing, debugging connectivity, understanding data formats

```bash
cargo run --bin test_market_data
```

## Configuration

### Environment Variables

Create a `.env` file in the trading bot directory:

```bash
# Twilight Protocol Configuration
CHAIN_ID=twilight-testnet
RPC_ENDPOINT=https://rpc.testnet.twilight.org:443
RELAYER_API_ENDPOINT=http://localhost:8088/api
RELAYER_PROGRAM_PATH=./relayerprogram.json

# Trading Configuration
DEFAULT_GAS=200000
DEFAULT_FEE=1000
TRADING_FEE_RATE=0.001

# Risk Management
MAX_POSITION_SIZE=50000
MAX_DAILY_LOSS=10000
EMERGENCY_STOP_THRESHOLD=0.1

# Logging
RUST_LOG=info
LOG_TO_FILE=true
LOG_DIRECTORY=./logs

# Development
PAPER_TRADING=true
ENABLE_BACKTESTING=false
```

### Wallet Setup

The bots will automatically create a new wallet on first run, or you can import an existing one:

```bash
# Create new wallet (mnemonic will be displayed once)
cargo run --bin main

# Import existing wallet via environment
export WALLET_MNEMONIC="your twelve word mnemonic phrase here..."
cargo run --bin main
```

## Usage Examples

### Basic Trading Bot

**Start with momentum strategy in paper trading mode:**

```bash
cargo run --bin main -- \
  --strategy momentum \
  --initial-capital 50000 \
  --max-leverage 5 \
  --paper-trading \
  --trading-interval 60
```

### Market Making Bot

**Run market maker with tight spreads:**

```bash
cargo run --bin simple_market_maker -- \
  --spread 0.001 \
  --order-size 2000 \
  --max-inventory 15000 \
  --refresh-interval 30
```

### Technical Analysis Bot

**Configure momentum trader with custom indicators:**

```bash
cargo run --bin momentum_trader -- \
  --fast-ma 5 \
  --slow-ma 20 \
  --rsi-period 21 \
  --stop-loss 0.03 \
  --take-profit 0.12 \
  --min-signal-strength 0.8
```

### Lending Bot

**Start automated lending with conservative settings:**

```bash
cargo run --bin lending_bot -- \
  --min-rate 0.08 \
  --max-exposure 0.6 \
  --lending-amount 5000 \
  --auto-reinvest \
  --monitoring-interval 300
```

## Strategy Details

### Momentum Trading Strategy

**Approach**: Identifies and follows market trends using technical indicators

**Key Components**:

- **Moving Averages**: Fast (10-period) vs Slow (30-period) crossovers
- **RSI**: Relative Strength Index for overbought/oversold conditions
- **Momentum**: Price rate of change for trend strength
- **Signal Strength**: Composite scoring for trade confidence

**Entry Conditions**:

- Fast MA crosses above Slow MA (bullish)
- RSI between 30-70 (not extreme)
- Strong momentum (>0.7 signal strength)
- Trend confirmation across indicators

**Exit Conditions**:

- Signal reversal (opposite trend detected)
- Stop loss (5% default)
- Take profit (15% default)
- Time-based exit (30-minute maximum hold)

### Market Making Strategy

**Approach**: Provides liquidity by placing simultaneous buy/sell orders with real-time price data

**Key Components**:

- **Real-time Price Data**: Fetches current BTC/USD prices from relayer API
- **Enhanced Market Analysis**: Optional order book and recent trades analysis
- **Spread Management**: Dynamic spreads based on volatility
- **Inventory Control**: Balances long/short exposure
- **Order Refreshing**: Updates orders based on market movement
- **Risk Management**: Hedges excessive inventory

**Order Placement**:

- Buy orders: Market price - (spread/2)
- Sell orders: Market price + (spread/2)
- Size: Fixed order size with inventory limits
- Duration: Orders refreshed every 60 seconds

**Inventory Management**:

- Maximum inventory: Configurable limit
- Hedging: Automatic hedge orders when limits exceeded
- Rebalancing: Adjusts order placement based on current inventory

### Lending Strategy

**Approach**: Provides liquidity to lending markets for yield generation

**Key Components**:

- **Rate Monitoring**: Tracks current lending rates
- **Exposure Management**: Controls maximum lending exposure
- **Auto-Reinvestment**: Compounds earned interest
- **Duration Management**: Optimizes lending periods

**Lending Decisions**:

- Rate threshold: Only lend above minimum rate
- Market analysis: Considers utilization and demand
- Position sizing: Based on available capital and exposure limits
- Duration: Typically 24-hour lending periods

## Risk Management

### Position Sizing

All bots implement sophisticated position sizing:

```rust
// Example: Risk-based position sizing
let risk_amount = account_balance * risk_per_trade;
let position_size = risk_amount / stop_loss_distance;
let max_position = account_balance * max_position_percentage;
let final_size = position_size.min(max_position);
```

### Stop Losses and Take Profits

**Automatic Protection**:

- **Stop Losses**: Limit downside risk (default 5%)
- **Take Profits**: Lock in gains (default 15%)
- **Trailing Stops**: Move stops in profitable direction
- **Time Stops**: Exit positions after maximum duration

### Leverage Management

**Conservative Approach**:

- Maximum leverage limits per strategy
- Dynamic leverage based on signal strength
- Lower leverage for uncertain signals
- Leverage scaling with volatility

### Error Handling

**Robust Error Recovery**:

- Automatic retry on network failures
- Graceful degradation during outages
- Position monitoring during reconnection
- Emergency stop functionality

## Best Practices

### Development Workflow

1. **Start with Paper Trading**:

   ```bash
   cargo run --bin main -- --paper-trading
   ```

2. **Test with Small Capital**:

   ```bash
   cargo run --bin main -- --initial-capital 1000
   ```

3. **Monitor Closely Initially**:

   ```bash
   RUST_LOG=debug cargo run --bin main
   ```

4. **Gradually Increase Capital**:
   - Start small and scale based on performance
   - Track all metrics and adjust parameters

### Performance Optimization

**Monitoring**:

- Use comprehensive logging
- Track win rates and P&L
- Monitor execution latency
- Analyze strategy performance

**Tuning**:

- Adjust parameters based on market conditions
- Backtest strategy changes
- Consider market regime changes
- Regular strategy review and updates

### Security Considerations

**Key Management**:

- Store mnemonics securely
- Use environment variables for sensitive data
- Consider hardware wallet integration
- Regular security audits

**Network Security**:

- Use secure RPC endpoints
- Monitor for unusual activity
- Implement rate limiting
- Consider VPN for trading operations

## Troubleshooting

### Common Issues

**1. Insufficient Balance**

```
Error: Insufficient balance. Required: 50000 sats, Available: 30000 sats
```

**Solution**: Fund your wallet or reduce initial capital

**2. Network Connection Issues**

```
Error: Failed to connect to relayer endpoint
```

**Solution**: Check RPC endpoints and network connectivity

**3. Order Execution Failures**

```
Error: Failed to open trader order: Invalid leverage
```

**Solution**: Check leverage limits and account balance

**4. Database Errors** (if using persistence features)

```
Error: Failed to initialize database
```

**Solution**: Check file permissions and SQLite installation

### Performance Issues

**High Latency**:

- Check network connection
- Consider closer RPC endpoints
- Optimize order frequency

**Memory Usage**:

- Monitor price history size
- Clean up old data periodically
- Consider data compression

### Debugging Commands

**Enable Debug Logging**:

```bash
RUST_LOG=debug cargo run --bin main
```

**Test Network Connectivity**:

```bash
curl -X POST -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"health","id":1}' \
  $RELAYER_API_ENDPOINT
```

**Check Wallet Status**:

```bash
cargo run --bin main -- --help
# Will show current balance and configuration
```

## Advanced Configuration

### Multiple Strategy Execution

Run multiple bots simultaneously:

```bash
# Terminal 1: Market maker
cargo run --bin simple_market_maker &

# Terminal 2: Momentum trader
cargo run --bin momentum_trader &

# Terminal 3: Lending bot
cargo run --bin lending_bot &
```

### Custom Strategy Development

Extend the existing bots by:

1. **Adding new indicators**:

   ```rust
   impl TechnicalIndicators {
       fn calculate_macd(&self) -> Option<f64> {
           // Custom MACD implementation
       }
   }
   ```

2. **Creating new strategies**:

   ```rust
   enum TradingStrategy {
       // Add new strategy
       Arbitrage,
       GridTrading,
       DollarCostAveraging,
   }
   ```

3. **Implementing risk models**:
   ```rust
   fn calculate_var_risk(&self) -> f64 {
       // Value at Risk calculation
   }
   ```

## Support and Contributing

### Getting Help

- **Documentation**: Check the main Nyks Wallet documentation
- **Issues**: Report bugs and request features via GitHub issues
- **Community**: Join the Twilight Protocol Discord for discussions

### Contributing

1. Fork the repository
2. Create a feature branch
3. Implement your changes with tests
4. Submit a pull request with detailed description

### License

This project is licensed under the same terms as the main Nyks Wallet project. See LICENSE file for details.

---

**⚠️ Disclaimer**: These examples are for educational purposes. Always test strategies thoroughly before using real capital. Trading involves risk of loss.
