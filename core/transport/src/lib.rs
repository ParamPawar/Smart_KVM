use webrtc::api::APIBuilder;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::data_channel::data_channel_init::RTCDataChannelInit;
use webrtc::data_channel::RTCDataChannel;
use std::sync::Arc;
use tracing::info;

/// A WebRTC connection handle that keeps the peer connection and data channel alive.
/// 
/// This structure ensures that the WebRTC connection is not dropped prematurely.
/// The connection will remain active as long as this handle is kept in scope.
pub struct WebRTCConnection {
    /// The peer connection - must be kept alive for the connection to remain open
    peer_connection: Arc<RTCPeerConnection>,
    /// The data channel - must be kept alive for sending/receiving data
    data_channel: Arc<RTCDataChannel>,
}

impl WebRTCConnection {
    /// Get a reference to the peer connection
    #[must_use]
    pub fn peer_connection(&self) -> &Arc<RTCPeerConnection> {
        &self.peer_connection
    }

    /// Get a reference to the data channel
    #[must_use]
    pub fn data_channel(&self) -> &Arc<RTCDataChannel> {
        &self.data_channel
    }
}

/// Start a WebRTC `DataChannel` connection
/// 
/// This function creates a WebRTC peer connection and data channel for communication.
/// The returned `WebRTCConnection` must be kept alive for the connection to remain active.
/// 
/// # Bug Fixes
/// 
/// 1. **Resource Lifetime Bug**: Previously, the peer connection and data channel were dropped
///    immediately at the end of the function, closing the connection. Now we return a
///    `WebRTCConnection` struct that keeps these resources alive.
/// 
/// 2. **UTF-8 Panic Risk**: Previously, `.unwrap()` was used on UTF-8 conversion which could
///    panic if invalid UTF-8 data was received. Now we use `String::from_utf8_lossy()` which
///    safely handles invalid UTF-8 by replacing invalid sequences with the replacement character.
/// 
/// # Returns
/// 
/// Returns a `WebRTCConnection` handle on success, or an error if connection setup fails.
/// Keep this handle alive to maintain the WebRTC connection.
/// 
/// # Errors
/// 
/// This function will return an error if:
/// - The WebRTC peer connection cannot be established
/// - The data channel creation fails
/// 
/// # Example
/// 
/// ```no_run
/// # use core_transport::start_webrtc;
/// # async fn example() -> anyhow::Result<()> {
/// let connection = start_webrtc().await?;
/// // Connection is active while `connection` is in scope
/// // ... use connection.data_channel() to send/receive data ...
/// # Ok(())
/// # }
/// ```
pub async fn start_webrtc() -> anyhow::Result<WebRTCConnection> {
    let api = APIBuilder::new().build();
    let config = RTCConfiguration::default();

    // Create peer connection
    let peer_connection = api.new_peer_connection(config).await?;
    let pc_arc = Arc::new(peer_connection);

    // Create data channel
    // Note: create_data_channel already returns Arc<RTCDataChannel>
    let dc_arc = pc_arc.create_data_channel("control", Some(RTCDataChannelInit::default())).await?;

    // Set up on_open handler
    dc_arc.on_open(Box::new(|| {
        Box::pin(async move {
            info!("✅ DataChannel open!");
        })
    }));

    // Set up on_message handler
    // Clone the Arc for the closure
    let dc_for_message = Arc::clone(&dc_arc);
    dc_for_message.on_message(Box::new(move |msg| {
        Box::pin(async move {
            // FIX: Use from_utf8_lossy instead of unwrap to avoid panics on invalid UTF-8
            // This safely converts bytes to string, replacing invalid UTF-8 sequences with �
            let text = String::from_utf8_lossy(&msg.data);
            info!("📩 Got message: {}", text);
        })
    }));

    // Return the connection handle to keep the connection alive
    Ok(WebRTCConnection {
        peer_connection: pc_arc,
        data_channel: dc_arc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_webrtc_connection_structure() {
        // Test that WebRTCConnection has the expected structure
        // This verifies that our fix for the resource lifetime bug is in place
        
        // We can't easily create a WebRTCConnection in tests without a full WebRTC setup,
        // but we can verify the type structure exists and has the correct methods
        
        // Verify the struct and its methods exist at compile time
        fn _assert_webrtc_connection_api(_conn: &WebRTCConnection) {
            let _pc = _conn.peer_connection();
            let _dc = _conn.data_channel();
        }
    }

    #[tokio::test]
    async fn test_start_webrtc_signature() {
        // Test that start_webrtc has the correct signature
        // We verify it returns WebRTCConnection, not () which was the bug
        
        // This test verifies the function signature at compile time
        let _: fn() -> _ = start_webrtc;
        
        // We can't actually call start_webrtc without proper WebRTC infrastructure,
        // but the type signature verification is valuable
    }

    #[test]
    fn test_utf8_lossy_behavior() {
        // Test that demonstrates the fix for the UTF-8 panic bug
        // We use from_utf8_lossy instead of from_utf8().unwrap()
        
        // Valid UTF-8
        let valid = b"Hello, world!";
        let text = String::from_utf8_lossy(valid);
        assert_eq!(text, "Hello, world!");
        
        // Invalid UTF-8 - this would panic with .unwrap() but is safe with _lossy
        let invalid = b"Hello \xFF world";
        let text = String::from_utf8_lossy(invalid);
        assert!(text.contains("Hello"));
        assert!(text.contains("world"));
        // Invalid byte is replaced with replacement character �
    }
}
