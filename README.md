# daily-logger

A Rust logging library that provides daily file rotation and colored console output with support for order-specific logging.

## Features

- **Daily log rotation**: Automatically creates separate log files for each day
- **Order-specific logging**: Support for UUID-based order logs
- **Colored console output**: Different colors for different log levels
- **Dual output**: Simultaneous logging to console and files with separate level controls
- **File caching**: Efficient file handle management with LRU cache
- **Thread-safe**: Built with concurrency in mind

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
daily-logger = "0.1.0"
```

## Usage

### Basic Setup

```rust
use daily_logger::init_logger;
use log::{info, warn, error, debug, trace};

fn main() {
    // Initialize logger with console level, file level, and base path
    init_logger(
        log::LevelFilter::Info,  // Console output level
        log::LevelFilter::Debug, // File output level  
        "/path/to/logs"          // Base directory for log files
    );

    // Use standard log macros
    info!("Application started");
    warn!("This is a warning");
    error!("An error occurred");
}
```

### Order-Specific Logging

The logger supports special UUID-based logging for tracking specific orders or transactions:

```rust
use log::info;

fn main() {
    daily_logger::init_logger(
        log::LevelFilter::Info,
        log::LevelFilter::Info,
        "/var/logs"
    );

    let order_id = "123e4567-e89b-12d3-a456-426614174000";
    
    // This will create a separate log file: order_123e4567-e89b-12d3-a456-426614174000.log
    info!(target: "orders", uuid = order_id; "Order processing started");
    info!(target: "orders", uuid = order_id; "Payment validated");
    info!(target: "orders", uuid = order_id; "Order completed");
}
```

## Log Output Formats

### Console Output
Console logs are colored by level and include timestamps:
```
2024-01-15T10:30:45+00:00-INFO|[orders]<123e4567>:Order processing started
```

### File Output
Files are created in the base directory:
- **Daily logs**: `log_2024_1_15.log` (contains all logs for the day)
- **Order logs**: `order_123e4567-e89b-12d3-a456-426614174000.log` (UUID-specific logs)

## Log Levels

The library supports standard log levels with color coding:

| Level | Color  | Description |
|-------|--------|-------------|
| ERROR | Red    | Error conditions |
| WARN  | Yellow | Warning conditions |
| INFO  | Green  | Informational messages |
| DEBUG | Blue   | Debug information |
| TRACE | Gray   | Trace information |

## Configuration

### Level Filtering

You can set different log levels for console and file output:

```rust
daily_logger::init_logger(
    log::LevelFilter::Warn,  // Only warnings and errors to console
    log::LevelFilter::Trace, // All levels to files
    "/var/logs"
);
```

### File Management

The logger uses an internal file cache (max 32 files) with LRU eviction to efficiently manage file handles. Log directories are created automatically if they don't exist.

## Dependencies

- `chrono` - Date and time handling
- `log` - Logging facade
- `once_cell` - Thread-safe lazy initialization
- `dashmap` - Concurrent hash map (if used in your implementation)

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.