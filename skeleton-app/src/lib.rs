// HYPERWARE SKELETON APP
// This is a minimal, well-commented skeleton app for the Hyperware platform
// using the Hyperapp framework (macro-driven approach).

// CRITICAL IMPORTS - DO NOT MODIFY THESE
// The hyperprocess_macro provides everything you need including:
// - Async/await support (custom runtime)
// - Automatic WIT (WebAssembly Interface Types) generation
// - State persistence
// - HTTP/WebSocket bindings
use hyperprocess_macro::*;

// HYPERWARE PROCESS LIB IMPORTS
// These are provided by the hyperprocess_macro, DO NOT add hyperware_process_lib to Cargo.toml
use hyperware_process_lib::{
    our,                    // Gets current node/process identity
    Address,                // For P2P addressing
    ProcessId,              // Process identifiers
    Request,                // For making requests to other processes/nodes
    homepage::add_to_homepage,  // Adds app icon to Hyperware homepage
};

// Standard imports for serialization
use serde::{Deserialize, Serialize};

// STEP 1: DEFINE YOUR APP STATE
// This struct holds all persistent data for your app
// It MUST derive Default, Serialize, and Deserialize
// Add PartialEq if you use this type in WIT interfaces
#[derive(Default, Serialize, Deserialize)]
pub struct AppState {
    // Example fields - replace with your app's data
    counter: u32,
    messages: Vec<String>,
    // For P2P apps, you might store:
    // my_node_id: Option<String>,
    // connected_nodes: Vec<String>,
}

// STEP 2: IMPLEMENT YOUR APP LOGIC
// The #[hyperprocess] attribute goes HERE, before the impl block
#[hyperprocess(
    // App name shown in the UI and logs
    name = "Skeleton App",
    
    // Enable UI serving at root path
    ui = Some(HttpBindingConfig::default()),
    
    // HTTP API endpoints - MUST include /api for frontend communication
    endpoints = vec![
        Binding::Http { 
            path: "/api", 
            config: HttpBindingConfig::new(false, false, false, None) 
        }
    ],
    
    // State persistence options:
    // - EveryMessage: Save after each message (safest, slower)
    // - OnInterval(n): Save every n seconds
    // - Never: No automatic saves (manual only)
    save_config = SaveOptions::EveryMessage,
    
    // WIT world name - must match package naming convention
    wit_world = "skeleton-app-dot-os-v0"
)]
impl AppState {
    // INITIALIZATION FUNCTION
    // Runs once when your process starts
    // Use this to:
    // - Register with the homepage
    // - Set up initial state
    // - Connect to other system processes
    #[init]
    async fn initialize(&mut self) {
        // Add your app to the Hyperware homepage
        // Parameters: name, icon (emoji), path, widget
        add_to_homepage("Skeleton App", Some("🦴"), Some("/"), None);
        
        // Initialize your app state
        self.counter = 0;
        self.messages.push("App initialized!".to_string());
        
        // Get our node identity (useful for P2P apps)
        let our_node = our().node.clone();
        println!("Skeleton app initialized on node: {}", our_node);
    }
    
    // HTTP ENDPOINT EXAMPLE
    // CRITICAL: ALL HTTP endpoints MUST accept _request_body parameter
    // even if they don't use it. This is a framework requirement.
    #[http]
    async fn get_status(&self, _request_body: String) -> String {
        // Return current app status as JSON
        serde_json::json!({
            "counter": self.counter,
            "message_count": self.messages.len(),
            "node": our().node
        }).to_string()
    }
    
    // HTTP ENDPOINT WITH PARAMETERS
    // Frontend sends parameters as either:
    // - Single value: { "MethodName": value }
    // - Multiple values as tuple: { "MethodName": [val1, val2] }
    #[http]
    async fn increment_counter(&mut self, request_body: String) -> Result<u32, String> {
        // Parse the increment amount from request
        let amount: u32 = match serde_json::from_str(&request_body) {
            Ok(val) => val,
            Err(_) => 1, // Default increment
        };
        
        self.counter += amount;
        self.messages.push(format!("Counter incremented by {}", amount));
        
        Ok(self.counter)
    }
    
    // HTTP ENDPOINT RETURNING COMPLEX DATA
    // For complex types, return as JSON string to avoid WIT limitations
    #[http]
    async fn get_messages(&self, _request_body: String) -> String {
        serde_json::to_string(&self.messages).unwrap_or_else(|_| "[]".to_string())
    }
    
    // REMOTE ENDPOINT EXAMPLE
    // These are called by other nodes in the P2P network
    // Use #[remote] instead of #[http]
    #[remote]
    async fn handle_remote_message(&mut self, message: String) -> Result<String, String> {
        // Store the message
        // Note: In remote handlers, you can't easily get the sender's node ID
        // You would need to include it in the message payload
        self.messages.push(format!("Remote message: {}", message));
        
        Ok("Message received".to_string())
    }
    
    // P2P COMMUNICATION EXAMPLE
    // Shows how to send messages to other nodes
    #[http]
    async fn send_to_node(&mut self, request_body: String) -> Result<String, String> {
        // Parse request containing target node and message
        #[derive(Deserialize)]
        struct SendRequest {
            target_node: String,
            message: String,
        }
        
        let req: SendRequest = serde_json::from_str(&request_body)
            .map_err(|e| format!("Invalid request: {}", e))?;
        
        // Construct the target address
        // Format: "process-name:package-name:publisher"
        let target_process_id = "skeleton-app:skeleton-app:skeleton.os"
            .parse::<ProcessId>()
            .map_err(|e| format!("Invalid process ID: {}", e))?;
        
        let target_address = Address::new(req.target_node, target_process_id);
        
        // Create request wrapper for remote method
        let request_wrapper = serde_json::json!({
            "HandleRemoteMessage": req.message
        });
        
        // Send the request
        // CRITICAL: Always set expects_response timeout for remote calls
        let result = Request::new()
            .target(target_address)
            .body(serde_json::to_vec(&request_wrapper).unwrap())
            .expects_response(30) // 30 second timeout
            .send_and_await_response(30);
        
        match result {
            Ok(_) => Ok("Message sent successfully".to_string()),
            Err(e) => Err(format!("Failed to send message: {:?}", e))
        }
    }
}

// ICON FOR YOUR APP (base64 encoded PNG, 256x256 recommended)
// Generate your own icon and encode it, or use an emoji in add_to_homepage
const ICON: &str = "";

// WIT TYPE COMPATIBILITY NOTES:
// The hyperprocess macro generates WebAssembly Interface Types from your code.
// Supported types:
// ✅ Primitives: bool, u8-u64, i8-i64, f32, f64, String
// ✅ Vec<T> where T is supported
// ✅ Option<T> where T is supported  
// ✅ Simple structs with public fields
// ❌ HashMap - use Vec<(K,V)> instead
// ❌ Fixed arrays [T; N] - use Vec<T>
// ❌ Complex enums with data
// 
// Workaround: Return complex data as JSON strings

// COMMON PATTERNS:

// 1. STATE MANAGEMENT
// Your AppState is automatically persisted based on save_config
// Access current state with &self (read) or &mut self (write)

// 2. ERROR HANDLING
// Return Result<T, String> for fallible operations
// The String error will be sent to the frontend

// 3. FRONTEND COMMUNICATION
// Frontend calls HTTP endpoints via POST to /api
// Body format: { "MethodName": parameters }

// 4. P2P PATTERNS
// - Use #[remote] for methods other nodes can call
// - Use Request API for calling other nodes
// - Always set timeouts for remote calls
// - Design for eventual consistency

// 5. SYSTEM INTEGRATION
// Common system processes you might interact with:
// - "vfs:distro:sys" - Virtual file system
// - "http-server:distro:sys" - HTTP server (automatic with macro)
// - "timer:distro:sys" - Timers and scheduling
// - "kv:distro:sys" - Key-value storage

// DEVELOPMENT WORKFLOW:
// 1. Define your AppState structure
// 2. Add HTTP endpoints for UI interaction
// 3. Add remote endpoints for P2P features
// 4. Build with: kit b --hyperapp
// 5. Start with: kit s
// 6. Access at: http://localhost:8080

// DEBUGGING TIPS:
// - Use println! for backend logs (appears in terminal)
// - Check browser console for frontend errors
// - Common issues:
//   * Missing _request_body parameter
//   * Wrong parameter format (object vs tuple)
//   * ProcessId parsing errors
//   * Missing /our.js in HTML