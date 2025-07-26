//! String interning for frequently used operator strings and keywords
//! 
//! This module provides string interning to reduce memory usage and improve
//! comparison performance for commonly used strings in the parser.

use once_cell::sync::Lazy;
use string_interner::{DefaultSymbol, StringInterner, DefaultBackend};
use std::sync::RwLock;

pub type InternedString = DefaultSymbol;
pub type DefaultStringInterner = StringInterner<DefaultBackend>;

/// Global interner for operator strings
/// 
/// Pre-populated with the most common operators to ensure they are
/// always available for fast lookups.
static OPERATOR_INTERNER: Lazy<RwLock<DefaultStringInterner>> = Lazy::new(|| {
    let mut interner = DefaultStringInterner::new();
    
    // Pre-intern the most common single-char operators
    interner.get_or_intern("+");
    interner.get_or_intern("-");
    interner.get_or_intern("*");
    interner.get_or_intern("/");
    interner.get_or_intern("=");
    interner.get_or_intern("^");
    interner.get_or_intern("(");
    interner.get_or_intern(")");
    interner.get_or_intern("[");
    interner.get_or_intern("]");
    interner.get_or_intern("{");
    interner.get_or_intern("}");
    
    // Pre-intern common multi-char operators
    interner.get_or_intern(":=");
    interner.get_or_intern("->");
    interner.get_or_intern("==");
    interner.get_or_intern("!=");
    interner.get_or_intern("<=");
    interner.get_or_intern(">=");
    interner.get_or_intern("&&");
    interner.get_or_intern("||");
    interner.get_or_intern("<>");
    interner.get_or_intern("~~");
    interner.get_or_intern(":>");
    interner.get_or_intern("/@");
    interner.get_or_intern("//");
    interner.get_or_intern("@@@");
    interner.get_or_intern("@@");
    interner.get_or_intern("@");
    
    RwLock::new(interner)
});

/// Intern an operator string using the global interner
/// 
/// This function uses a read-write lock pattern for optimal performance:
/// - Try read lock first for common strings already in the interner
/// - Fall back to write lock only for new strings
pub fn intern_operator(s: &str) -> InternedString {
    // Try read lock first for common case
    if let Ok(interner) = OPERATOR_INTERNER.read() {
        if let Some(symbol) = interner.get(s) {
            return symbol;
        }
    }
    
    // Fall back to write lock for new strings
    OPERATOR_INTERNER.write().unwrap().get_or_intern(s)
}

/// Resolve an interned string back to its string representation
/// 
/// # Panics
/// 
/// Panics if the symbol is invalid (not from this interner).
pub fn resolve_interned(symbol: InternedString) -> String {
    OPERATOR_INTERNER.read().unwrap()
        .resolve(symbol)
        .expect("Invalid interned symbol")
        .to_string()
}

/// Try to resolve an interned string, returning None if invalid
pub fn try_resolve_interned(symbol: InternedString) -> Option<String> {
    OPERATOR_INTERNER.read().ok()?
        .resolve(symbol)
        .map(|s| s.to_string())
}

/// Thread-local interner for single-threaded parsing contexts
/// 
/// This provides better performance for single-threaded scenarios by
/// avoiding the overhead of RwLock synchronization.
#[derive(Debug)]
pub struct LocalInterner {
    interner: DefaultStringInterner,
}

impl LocalInterner {
    /// Create a new local interner pre-populated with common strings
    pub fn new() -> Self {
        let mut interner = DefaultStringInterner::new();
        Self::prepopulate(&mut interner);
        Self { interner }
    }
    
    /// Pre-populate the interner with commonly used strings
    fn prepopulate(interner: &mut DefaultStringInterner) {
        // Common single-char operators
        for op in &["+", "-", "*", "/", "=", "^", "(", ")", "[", "]", "{", "}"] {
            interner.get_or_intern(op);
        }
        
        // Common multi-char operators
        for op in &[":=", "->", "==", "!=", "<=", ">=", "&&", "||", "<>", "~~", 
                   ":>", "/@", "//", "@@@", "@@", "@"] {
            interner.get_or_intern(op);
        }
        
        // Common function names
        for func in &["Plus", "Times", "Power", "List", "Set", "Rule", "If", 
                     "While", "Module", "Block", "Function", "Apply", "Map"] {
            interner.get_or_intern(func);
        }
    }
    
    /// Intern a string using this local interner
    pub fn intern(&mut self, s: &str) -> InternedString {
        self.interner.get_or_intern(s)
    }
    
    /// Resolve an interned string from this local interner
    pub fn resolve(&self, symbol: InternedString) -> &str {
        self.interner.resolve(symbol).unwrap()
    }
    
    /// Get the number of strings in this interner
    pub fn len(&self) -> usize {
        self.interner.len()
    }
    
    /// Check if the interner should be cleaned up due to size
    pub fn should_cleanup(&self) -> bool {
        self.interner.len() > 10_000 // Threshold for cleanup
    }
    
    /// Clean up the interner by resetting it and keeping only common strings
    pub fn cleanup(&mut self) {
        let mut new_interner = DefaultStringInterner::new();
        Self::prepopulate(&mut new_interner);
        self.interner = new_interner;
    }
}

impl Default for LocalInterner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_global_interner() {
        let symbol1 = intern_operator("+");
        let symbol2 = intern_operator("+");
        
        // Same string should return same symbol
        assert_eq!(symbol1, symbol2);
        
        // Should resolve back to original string
        assert_eq!(resolve_interned(symbol1), "+");
    }
    
    #[test]
    fn test_local_interner() {
        let mut interner = LocalInterner::new();
        
        let symbol1 = interner.intern("test");
        let symbol2 = interner.intern("test");
        
        // Same string should return same symbol
        assert_eq!(symbol1, symbol2);
        
        // Should resolve back to original string
        assert_eq!(interner.resolve(symbol1), "test");
    }
    
    #[test]
    fn test_prepopulated_operators() {
        let interner = LocalInterner::new();
        
        // Check that common operators are pre-populated
        assert!(interner.len() > 20);
    }
    
    #[test]
    fn test_cleanup() {
        let mut interner = LocalInterner::new();
        let initial_len = interner.len();
        
        // Add many strings
        for i in 0..100 {
            interner.intern(&format!("test_{}", i));
        }
        
        assert!(interner.len() > initial_len + 90);
        
        // Cleanup should reset to just the pre-populated strings
        interner.cleanup();
        assert_eq!(interner.len(), initial_len);
    }
}