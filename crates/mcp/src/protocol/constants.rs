pub const JSONRPC_VERSION: &str = "2.0";
pub const LATEST_PROTOCOL_VERSION: &str = "2024-11-05";

// Standard JSON-RPC error codes
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;