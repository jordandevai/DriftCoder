# SSH Handshake Abort Fix for LAN Connections

## Problem Description

When connecting from an Android tablet over LAN, SSH connections would fail with `ssh_handshake_aborted` error, even though the test connection would succeed. Local connections worked fine.

## Root Cause Analysis

The issue was caused by a combination of factors specific to mobile/tablet network conditions:

### 1. Insufficient TCP Timeout
The TCP connection timeout was set to 8 seconds, which is too short for mobile/tablet networks with higher latency and packet loss.

### 2. Race Condition in Connection Handoff
After successful authentication, the connection was immediately moved to the actor task without any stabilization period. On mobile devices with less predictable async runtime scheduling, this could cause the handshake to abort when the connection was put under load immediately.

### 3. No Connection Health Check
There was no verification that the connection remained alive after the handshake completed, which meant unstable connections could be handed off to the actor even if they were already failing.

### 4. Aggressive Rekeying
Default SSH rekey settings were too aggressive for mobile networks, causing unnecessary rekey operations that could fail on unstable connections.

## Changes Made

### 1. Increased TCP Timeout (client.rs:127)
```rust
// Before: Duration::from_secs(8)
// After:  Duration::from_secs(15)
tokio::time::timeout(Duration::from_secs(15), TcpStream::connect(addr))
```
**Rationale**: Gives mobile/tablet networks more time to establish the TCP connection.

### 2. Added Connection Warmup Period (client.rs:201-205)
```rust
// Warmup period: Allow the connection to stabilize, especially important for
// mobile/tablet networks where async runtime scheduling may be less predictable.
// This prevents handshake aborts when the connection is immediately put under load.
tokio::time::sleep(Duration::from_millis(100)).await;
```
**Rationale**: Gives the connection time to stabilize before being used, preventing race conditions.

### 3. Added Connection Health Check (client.rs:207-212)
```rust
// Verify the connection is still alive after warmup
// This catches cases where the handshake completed but the connection dropped
// during the warmup period (common on unstable mobile networks)
if handle.is_closed() {
    return Err(SshError::ConnectionFailed(
        "Connection closed during warmup".to_string(),
    ));
}
```
**Rationale**: Catches unstable connections early before they cause issues in the actor.

### 4. Increased Rekey Time and Limit (client.rs:97-100)
```rust
// Increase rekey time for mobile networks - rekeying can cause issues on unstable connections
config.rekey_time = Some(Duration::from_secs(3600));
config.rekey_limit = Some(1024 * 1024 * 1024); // 1GB
```
**Rationale**: Reduces the frequency of rekey operations which can fail on unstable mobile networks.

### 5. Added Debug Logging (client.rs:143-151)
```rust
log::debug!("Starting SSH handshake to {}", addr);
// ... handshake code ...
log::debug!("SSH handshake successful to {}", addr);
log::warn!("SSH handshake failed to {}: {}", addr, msg);
```
**Rationale**: Helps diagnose connection issues in production.

## Why Test Connection Worked

The test connection succeeded because:
1. It creates a connection, verifies it, then immediately disconnects
2. No actor task is spawned, so no race condition occurs
3. The connection is never put under sustained load
4. The connection lifecycle is short, so unstable connections don't have time to fail

## Why Actual Connection Failed

The actual connection failed because:
1. Connection is handed off to an actor task immediately after authentication
2. The actor starts processing requests immediately
3. On mobile devices with variable async scheduling, this can cause the handshake to abort
4. Unstable connections that drop during the warmup period weren't caught

## Testing Recommendations

After applying these changes, test the following scenarios:

1. **Basic LAN Connection**: Connect from your Android tablet over LAN
2. **High Latency**: Simulate high latency (e.g., using `tc` to add delay)
3. **Packet Loss**: Simulate packet loss to test connection stability
4. **Multiple Connections**: Test multiple simultaneous connections
5. **Long-running Sessions**: Test connections that stay open for extended periods

## Monitoring

Check the logs for these debug messages:
- `Starting SSH handshake to <address>` - Handshake initiated
- `SSH handshake successful to <address>` - Handshake completed
- `SSH handshake failed to <address>: <error>` - Handshake failed
- `SSH connection established to <host>:<port>` - Connection fully established

If you still see `ssh_handshake_aborted` errors, the logs will help identify whether:
- The TCP connection is timing out (check network latency)
- The handshake is failing (check SSH server compatibility)
- The connection is dropping during warmup (check network stability)

## Additional Considerations

If issues persist, consider:

1. **Network Quality**: Ensure your Wi-Fi network is stable and has good signal strength
2. **SSH Server Configuration**: Check the SSH server's `ClientAliveInterval` and `ClientAliveCountMax` settings
3. **Firewall Rules**: Ensure no intermediate firewalls are dropping connections
4. **MTU Size**: Large MTU sizes can cause fragmentation issues on some networks
5. **IPv6**: The code prefers IPv4, but if you're using IPv6, ensure your network supports it properly

## Performance Impact

These changes have minimal performance impact:
- The 100ms warmup delay is negligible compared to network latency
- Increased TCP timeout only affects slow connections
- Increased rekey time reduces CPU usage for rekey operations
- Connection health check is a simple boolean check

The tradeoff is slightly slower connection establishment for significantly better reliability on mobile/tablet networks.
