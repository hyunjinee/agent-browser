use thiserror::Error;

// ---------------------------------------------------------------------------
// Browser errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum BrowserError {
    #[error("Browser not launched")]
    NotLaunched,

    #[error("No active page. Open a URL first")]
    NoActivePage,

    #[error("Unknown engine '{engine}'. Supported engines: chrome, lightpanda")]
    UnknownEngine { engine: String },

    #[error("Chrome launch task failed: {0}")]
    LaunchTaskFailed(String),

    #[error("{0}")]
    Validation(String),

    #[error("CDP WebSocket connect failed: {0}")]
    ConnectionFailed(String),

    #[error("{message}")]
    Cdp { message: String },

    #[error("{0}")]
    Other(String),
}

impl From<String> for BrowserError {
    fn from(s: String) -> Self {
        BrowserError::Other(s)
    }
}

impl From<&str> for BrowserError {
    fn from(s: &str) -> Self {
        BrowserError::Other(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Authentication errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Invalid profile name '{name}'. Must match /^[a-zA-Z0-9_-]+$/")]
    InvalidProfileName { name: String },

    #[error("Profile '{name}' not found")]
    ProfileNotFound { name: String },

    #[error("Failed to {operation}: {detail}")]
    Io { operation: String, detail: String },

    #[error("{0}")]
    Encryption(String),

    #[error("{0}")]
    Other(String),
}

impl From<String> for AuthError {
    fn from(s: String) -> Self {
        AuthError::Other(s)
    }
}

impl From<&str> for AuthError {
    fn from(s: &str) -> Self {
        AuthError::Other(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// State management errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum StateError {
    #[error("Failed to {operation}: {detail}")]
    Io { operation: String, detail: String },

    #[error("State file not found: {path}")]
    NotFound { path: String },

    #[error("Session '{name}' not found")]
    SessionNotFound { name: String },

    #[error("{0}")]
    Serialization(String),

    #[error("{0}")]
    Other(String),
}

impl From<String> for StateError {
    fn from(s: String) -> Self {
        StateError::Other(s)
    }
}

impl From<&str> for StateError {
    fn from(s: &str) -> Self {
        StateError::Other(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Provider errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("{env_var} environment variable is not set")]
    MissingApiKey { env_var: String },

    #[error("Unknown provider '{name}'. Supported: browserbase, browserless, browser-use, kernel")]
    UnknownProvider { name: String },

    #[error("{provider} request failed: {message}")]
    RequestFailed { provider: String, message: String },

    #[error("{provider} returned HTTP {status}: {body}")]
    HttpError {
        provider: String,
        status: u16,
        body: String,
    },

    #[error("{0}")]
    Other(String),
}

impl From<String> for ProviderError {
    fn from(s: String) -> Self {
        ProviderError::Other(s)
    }
}

impl From<&str> for ProviderError {
    fn from(s: &str) -> Self {
        ProviderError::Other(s.to_string())
    }
}

// ---------------------------------------------------------------------------
// Element errors
// ---------------------------------------------------------------------------

#[derive(Debug, Error)]
pub enum ElementError {
    #[error("No element found for {strategy} '{value}'")]
    NotFound { strategy: String, value: String },

    #[error("Element matched multiple results. Use a more specific selector")]
    MultipleMatches,

    #[error("Element exists but is not visible")]
    NotVisible,

    #[error("{0}")]
    Other(String),
}

impl From<String> for ElementError {
    fn from(s: String) -> Self {
        ElementError::Other(s)
    }
}

impl From<&str> for ElementError {
    fn from(s: &str) -> Self {
        ElementError::Other(s.to_string())
    }
}
