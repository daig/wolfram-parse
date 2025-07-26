//! Newtypes for improved type safety and API ergonomics.

use std::num::NonZeroU32;

/// A validated tab width value that cannot be zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TabWidth(NonZeroU32);

impl TabWidth {
    /// Create a new TabWidth if the value is valid (non-zero).
    pub fn new(width: u32) -> Option<Self> {
        NonZeroU32::new(width).map(TabWidth)
    }
    
    /// Get the default tab width (4 spaces).
    pub fn default() -> Self {
        TabWidth(NonZeroU32::new(4).unwrap())
    }
    
    /// Extract the underlying value.
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

impl Default for TabWidth {
    fn default() -> Self {
        Self::default()
    }
}

/// A confidence level between 0.0 and 1.0 (inclusive).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ConfidenceLevel(f64);

impl ConfidenceLevel {
    /// Create a new ConfidenceLevel if the value is in the valid range [0.0, 1.0].
    pub fn new(level: f64) -> Option<Self> {
        if (0.0..=1.0).contains(&level) {
            Some(ConfidenceLevel(level))
        } else {
            None
        }
    }
    
    /// Create a confidence level representing absolute certainty (1.0).
    pub fn certain() -> Self {
        ConfidenceLevel(1.0)
    }
    
    /// Create a confidence level representing no confidence (0.0).
    pub fn none() -> Self {
        ConfidenceLevel(0.0)
    }
    
    /// Extract the underlying value.
    pub fn get(self) -> f64 {
        self.0
    }
}

/// A line number that cannot be zero (lines start at 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LineNumber(NonZeroU32);

impl LineNumber {
    /// Create a new LineNumber if the value is valid (non-zero).
    pub fn new(line: u32) -> Option<Self> {
        NonZeroU32::new(line).map(LineNumber)
    }
    
    /// Get the first line number (1).
    pub fn first() -> Self {
        LineNumber(NonZeroU32::new(1).unwrap())
    }
    
    /// Extract the underlying value.
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

/// A column number that cannot be zero (columns start at 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColumnNumber(NonZeroU32);

impl ColumnNumber {
    /// Create a new ColumnNumber if the value is valid (non-zero).
    pub fn new(col: u32) -> Option<Self> {
        NonZeroU32::new(col).map(ColumnNumber)
    }
    
    /// Get the first column number (1).
    pub fn first() -> Self {
        ColumnNumber(NonZeroU32::new(1).unwrap())
    }
    
    /// Extract the underlying value.
    pub fn get(self) -> u32 {
        self.0.get()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_tab_width_validation() {
        assert!(TabWidth::new(0).is_none());
        assert!(TabWidth::new(1).is_some());
        assert!(TabWidth::new(8).is_some());
        
        assert_eq!(TabWidth::default().get(), 4);
        assert_eq!(TabWidth::new(2).unwrap().get(), 2);
    }

    #[test]
    fn test_confidence_level_validation() {
        assert!(ConfidenceLevel::new(-0.1).is_none());
        assert!(ConfidenceLevel::new(0.0).is_some());
        assert!(ConfidenceLevel::new(0.5).is_some());
        assert!(ConfidenceLevel::new(1.0).is_some());
        assert!(ConfidenceLevel::new(1.1).is_none());
        
        assert_eq!(ConfidenceLevel::certain().get(), 1.0);
        assert_eq!(ConfidenceLevel::none().get(), 0.0);
    }

    #[test]
    fn test_line_number_validation() {
        assert!(LineNumber::new(0).is_none());
        assert!(LineNumber::new(1).is_some());
        
        assert_eq!(LineNumber::first().get(), 1);
        assert_eq!(LineNumber::new(42).unwrap().get(), 42);
    }

    #[test]
    fn test_column_number_validation() {
        assert!(ColumnNumber::new(0).is_none());
        assert!(ColumnNumber::new(1).is_some());
        
        assert_eq!(ColumnNumber::first().get(), 1);
        assert_eq!(ColumnNumber::new(80).unwrap().get(), 80);
    }
}