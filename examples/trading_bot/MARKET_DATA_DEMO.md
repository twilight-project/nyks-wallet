# Market Data Integration Demo

This document demonstrates the real-time market data integration in the Simple Market Maker bot using the Relayer API.

## Features Implemented

### 1. Basic Price Fetching

The market maker now fetches real BTC/USD prices from the relayer API instead of using simulated data:

```rust
// Basic price update from relayer API
let btc_price = order_wallet.relayer_api_client.btc_usd_price().await?;
let new_price = btc_price.price as u64;
```

### 2. Enhanced Market Data (Optional)

When using `--enhanced-market-data`, the bot fetches multiple data sources concurrently:

- **Current BTC/USD Price**: Real-time price from relayer
- **Order Book**: Current bid/ask prices for spread calculation
- **Recent Trades**: Volume and activity analysis

```bash
# Run with enhanced market data
cargo run --bin simple_market_maker -- --enhanced-market-data --paper-trading
```

## API Endpoints Used

The Simple Market Maker now integrates with these relayer API endpoints:

1. **`btc_usd_price()`** - Current BTC/USD price with timestamp
2. **`open_limit_orders()`** - Current order book (bids/asks)
3. **`recent_trade_orders()`** - Recent trading activity

## Price Calculation Logic

### Basic Mode

- Fetches current BTC price directly from relayer API
- Falls back to simulation if API call fails
- Logs price changes with percentage differences

### Enhanced Mode

- Fetches price, order book, and recent trades concurrently
- Calculates mid-market price from order book: `(best_bid + best_ask) / 2`
- Weights final price: `70% relayer price + 30% order book mid-price`
- Logs trading volume and market activity

## Testing the Integration

### 1. Test Basic Price Fetching

```bash
# Basic market maker with real price data
cargo run --bin simple_market_maker -- \
  --spread 0.002 \
  --order-size 1000 \
  --paper-trading \
  --refresh-interval 30
```

Expected output:

```
Market price updated from relayer: 45000 -> 45150 (change: 0.333%)
Market price unchanged at 45150 (timestamp: 2024-01-15T10:30:00Z)
```

### 2. Test Enhanced Market Data

```bash
# Enhanced market maker with order book analysis
cargo run --bin simple_market_maker -- \
  --enhanced-market-data \
  --paper-trading \
  --spread 0.001 \
  --refresh-interval 60
```

Expected output:

```
Latest BTC price: 45150 (timestamp: 2024-01-15T10:30:00Z)
Order book - Bid: 45140.50, Ask: 45159.50, Mid: 45150.00
Recent trading volume: 25000 sats from 15 orders
Enhanced market price update: 45150 -> 45150 (change: 0.000%)
```

### 3. Test Fallback Behavior

When the relayer API is unavailable, the bot gracefully falls back to simulation:

```
Failed to fetch price from relayer API: Connection refused, using simulation
Market price simulated: 45150 -> 45152 (fallback mode)
```

## Integration Benefits

### 1. Real Market Awareness

- Actual BTC prices instead of random simulation
- Market maker spreads based on real market conditions
- Better inventory management with true price movements

### 2. Enhanced Decision Making

- Order book analysis for optimal spread placement
- Volume analysis for market activity assessment
- Multiple data sources for robust price discovery

### 3. Risk Management

- Automatic fallback to simulation if API fails
- Price change validation and logging
- Graceful error handling for network issues

## Configuration Options

```bash
# Minimal configuration for testing
cargo run --bin simple_market_maker -- --paper-trading

# Production-ready configuration with enhanced data
cargo run --bin simple_market_maker -- \
  --enhanced-market-data \
  --spread 0.0015 \
  --order-size 5000 \
  --max-inventory 20000 \
  --refresh-interval 45 \
  --max-leverage 2

# High-frequency configuration
cargo run --bin simple_market_maker -- \
  --enhanced-market-data \
  --spread 0.0008 \
  --order-size 2000 \
  --refresh-interval 15
```

## API Error Handling

The implementation includes robust error handling:

1. **Network Failures**: Falls back to price simulation
2. **Invalid Responses**: Logs errors and continues operation
3. **Rate Limiting**: Respects API call frequency limits
4. **Partial Failures**: Enhanced mode continues with available data

## Performance Considerations

### Basic Mode

- Single API call per price update
- Minimal network overhead
- Fast execution (< 100ms typically)

### Enhanced Mode

- Three concurrent API calls using `tokio::join!`
- Higher network usage but more complete data
- Slightly higher latency (< 300ms typically)
- Better for less frequent updates (60s+ intervals)

## Future Enhancements

Potential improvements to the market data integration:

1. **Historical Price Analysis**: Use candle data for trend analysis
2. **Funding Rate Integration**: Adjust strategies based on funding rates
3. **Volume-Weighted Pricing**: Use volume data for better price estimation
4. **Market Depth Analysis**: Analyze order book depth for spread optimization
5. **WebSocket Integration**: Real-time price updates instead of polling

## Troubleshooting

Common issues and solutions:

### API Connection Issues

```
Error: Failed to fetch price from relayer API: Connection refused
```

**Solution**: Check relayer endpoint configuration and network connectivity

### Authentication Errors

```
Error: Failed to fetch price from relayer API: Unauthorized
```

**Solution**: Verify API credentials and endpoint permissions

### Rate Limiting

```
Error: Failed to fetch price from relayer API: Too many requests
```

**Solution**: Increase refresh interval or implement request throttling

### Invalid Data

```
Error: Failed to parse price data: Invalid JSON
```

**Solution**: Check API response format and update parsing logic
