/// Symbol mapping for Hyperliquid spot markets
///
/// Hyperliquid spot markets use special coin IDs (e.g., @107 for HYPE/USDC)
/// This module maps user-friendly names to Hyperliquid's internal format

use std::collections::HashMap;
use once_cell::sync::Lazy;

/// Map of spot market names to Hyperliquid coin IDs
static SPOT_SYMBOL_MAP: Lazy<HashMap<&'static str, &'static str>> = Lazy::new(|| {
    let mut m = HashMap::new();

    // HYPE spot market
    m.insert("HYPE/USDC", "@107");
    m.insert("HYPE", "@107");  // Allow both formats

    // PURR spot market
    m.insert("PURR/USDC", "PURR/USDC");  // Already in correct format
    m.insert("PURR", "PURR/USDC");

    // Add more as needed...

    m
});

/// Convert a user-friendly spot symbol to Hyperliquid's internal format
///
/// Examples:
/// - "HYPE/USDC" → "@107"
/// - "HYPE" → "@107"
/// - "PURR/USDC" → "PURR/USDC"
/// - "@107" → "@107" (pass-through)
pub fn map_spot_symbol(symbol: &str) -> String {
    // If it already starts with @, assume it's already in the correct format
    if symbol.starts_with('@') {
        return symbol.to_string();
    }

    // Try to find mapping
    SPOT_SYMBOL_MAP
        .get(symbol)
        .map(|&s| s.to_string())
        .unwrap_or_else(|| {
            // If no mapping found, return as-is (might be already correct)
            symbol.to_string()
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hype_mapping() {
        assert_eq!(map_spot_symbol("HYPE/USDC"), "@107");
        assert_eq!(map_spot_symbol("HYPE"), "@107");
        assert_eq!(map_spot_symbol("@107"), "@107");
    }

    #[test]
    fn test_purr_mapping() {
        assert_eq!(map_spot_symbol("PURR/USDC"), "PURR/USDC");
        assert_eq!(map_spot_symbol("PURR"), "PURR/USDC");
    }

    #[test]
    fn test_unknown_symbol() {
        // Unknown symbols are passed through
        assert_eq!(map_spot_symbol("UNKNOWN/USDC"), "UNKNOWN/USDC");
    }
}
