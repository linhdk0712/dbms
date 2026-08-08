//! Authentication & Authorization module.
//!
//! Provides JWT token management, OIDC login flow, role-based access control
//! extractors, auth middleware, rate limiting, and token blacklisting.

pub mod cookie;
pub mod extractors;
pub mod global_rate_limit;
pub mod jwt;
pub mod middleware;
pub mod oidc;
pub mod rate_limit;
pub mod token_blacklist;
