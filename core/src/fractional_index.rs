//! Fractional indexing utilities for deterministic ordering in local-first systems.
//!
//! This module provides utilities to generate fractional indices that maintain
//! lexicographic ordering and allow for conflict-free insertion of items at
//! arbitrary positions by different clients.

/// Characters used in fractional indices, ordered lexicographically
const CHARS: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";
const BASE: usize = CHARS.len();

/// Error types for fractional indexing operations
#[derive(Debug, Clone, PartialEq)]
pub enum FractionalIndexError {
    InvalidCharacter(char),
    InvalidIndex(String),
    CannotGenerate(String),
}

impl std::fmt::Display for FractionalIndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FractionalIndexError::InvalidCharacter(c) => {
                write!(f, "Invalid character in fractional index: {}", c)
            }
            FractionalIndexError::InvalidIndex(s) => {
                write!(f, "Invalid fractional index: {}", s)
            }
            FractionalIndexError::CannotGenerate(reason) => {
                write!(f, "Cannot generate fractional index: {}", reason)
            }
        }
    }
}

impl std::error::Error for FractionalIndexError {}

pub type Result<T> = std::result::Result<T, FractionalIndexError>;

/// Generate the first fractional index
pub fn initial() -> String {
    "a0".to_string()
}

/// Generate a fractional index between two existing indices
pub fn between(a: &str, b: &str) -> Result<String> {
    if a >= b {
        return Err(FractionalIndexError::CannotGenerate(format!(
            "First index '{}' must be less than second index '{}'",
            a, b
        )));
    }

    // Validate both indices
    validate_index(a)?;
    validate_index(b)?;

    // Convert to digit arrays for calculation
    let a_digits = to_digits(a)?;
    let b_digits = to_digits(b)?;

    // Find the midpoint
    let mid_digits = midpoint(&a_digits, &b_digits)?;

    // Convert back to string
    Ok(from_digits(&mid_digits))
}

/// Generate a fractional index before the given index
pub fn before(index: &str) -> Result<String> {
    validate_index(index)?;

    if index.is_empty() {
        return Ok("a0".to_string());
    }

    // If we can decrement the last character, do so
    let mut chars: Vec<char> = index.chars().collect();
    if let Some(last_char) = chars.last_mut() {
        if let Some(prev_char) = get_previous_char(*last_char) {
            *last_char = prev_char;
            return Ok(chars.into_iter().collect());
        }
    }

    // If we can't decrement, we need to go to the previous "level"
    // This is more complex, so we'll use a simpler approach
    // by finding midpoint between empty string and current index
    let empty_digits = vec![0]; // Represents empty/minimal index
    let index_digits = to_digits(index)?;
    let mid_digits = midpoint(&empty_digits, &index_digits)?;

    Ok(from_digits(&mid_digits))
}

/// Generate a fractional index after the given index
pub fn after(index: &str) -> Result<String> {
    validate_index(index)?;

    // Try to increment the last character
    let mut chars: Vec<char> = index.chars().collect();
    if let Some(last_char) = chars.last_mut() {
        if let Some(next_char) = get_next_char(*last_char) {
            *last_char = next_char;
            return Ok(chars.into_iter().collect());
        }
    }

    // If we can't increment, append a character
    chars.push(char_at(1)); // Append '1'
    Ok(chars.into_iter().collect())
}

/// Validate that a fractional index contains only valid characters
pub fn validate_index(index: &str) -> Result<()> {
    if index.is_empty() {
        return Err(FractionalIndexError::InvalidIndex(
            "Empty index".to_string(),
        ));
    }

    for c in index.chars() {
        if !is_valid_char(c) {
            return Err(FractionalIndexError::InvalidCharacter(c));
        }
    }
    Ok(())
}

/// Check if indices are in correct order
pub fn is_valid_order(indices: &[String]) -> bool {
    indices.windows(2).all(|w| w[0] < w[1])
}

/// Get the character at the given position in our character set
fn char_at(pos: usize) -> char {
    CHARS[pos % BASE] as char
}

/// Get the position of a character in our character set
fn char_pos(c: char) -> Option<usize> {
    CHARS.iter().position(|&ch| ch == c as u8)
}

/// Check if a character is valid for fractional indices
fn is_valid_char(c: char) -> bool {
    char_pos(c).is_some()
}

/// Get the previous character in our sequence
fn get_previous_char(c: char) -> Option<char> {
    char_pos(c).and_then(|pos| {
        if pos > 0 {
            Some(char_at(pos - 1))
        } else {
            None
        }
    })
}

/// Get the next character in our sequence
fn get_next_char(c: char) -> Option<char> {
    char_pos(c).and_then(|pos| {
        if pos < BASE - 1 {
            Some(char_at(pos + 1))
        } else {
            None
        }
    })
}

/// Convert a fractional index string to an array of digit positions
fn to_digits(index: &str) -> Result<Vec<usize>> {
    index
        .chars()
        .map(|c| char_pos(c).ok_or(FractionalIndexError::InvalidCharacter(c)))
        .collect()
}

/// Convert an array of digit positions back to a fractional index string
fn from_digits(digits: &[usize]) -> String {
    digits.iter().map(|&pos| char_at(pos)).collect()
}

/// Find the midpoint between two digit arrays
fn midpoint(a: &[usize], b: &[usize]) -> Result<Vec<usize>> {
    let max_len = a.len().max(b.len());
    let mut result = Vec::new();
    let _carry = 0;

    for i in 0..max_len {
        let a_digit = if i < a.len() { a[i] } else { 0 };
        let b_digit = if i < b.len() { b[i] } else { BASE - 1 };

        if a_digit == b_digit {
            result.push(a_digit);
            continue;
        }

        if a_digit + 1 == b_digit {
            // Adjacent digits - we need to go deeper
            result.push(a_digit);
            // Add a midpoint digit
            let mid = (BASE - 1) / 2;
            result.push(mid);
            break;
        } else {
            // Non-adjacent digits - we can find a midpoint
            let mid = (a_digit + b_digit) / 2;
            result.push(mid);
            break;
        }
    }

    // Ensure we have a valid result
    if result.is_empty() {
        result.push(BASE / 2);
    }

    Ok(result)
}

/// Generate a sequence of fractional indices for initial setup
pub fn generate_sequence(count: usize) -> Vec<String> {
    if count == 0 {
        return Vec::new();
    }

    let mut result = vec![initial()];

    for _ in 1..count {
        let last = result.last().unwrap();
        match after(last) {
            Ok(next) => result.push(next),
            Err(_) => {
                // Fallback: use base conversion
                let index = result.len();
                result.push(format!("z{}", index));
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initial() {
        let index = initial();
        assert_eq!(index, "a0");
        assert!(validate_index(&index).is_ok());
    }

    #[test]
    fn test_between_simple() {
        let result = between("a0", "b0").unwrap();
        assert!(result.as_str() > "a0" && result.as_str() < "b0");
        assert!(validate_index(&result).is_ok());
    }

    #[test]
    fn test_between_adjacent() {
        let result = between("a0", "a2").unwrap();
        assert!(result.as_str() > "a0" && result.as_str() < "a2");
        assert_eq!(result, "a1");
    }

    #[test]
    fn test_between_very_close() {
        let result = between("a0", "a1").unwrap();
        assert!(result.as_str() > "a0" && result.as_str() < "a1");
        // Should generate something like "a0V" (midpoint of remaining chars)
        assert!(result.starts_with("a0"));
    }

    #[test]
    fn test_before() {
        let result = before("b0").unwrap();
        assert!(result.as_str() < "b0");
        assert!(validate_index(&result).is_ok());
    }

    #[test]
    fn test_after() {
        let result = after("a0").unwrap();
        assert!(result.as_str() > "a0");
        assert!(validate_index(&result).is_ok());
    }

    #[test]
    fn test_ordering() {
        let indices = vec![
            "a0".to_string(),
            "a1".to_string(),
            "b0".to_string(),
            "c0".to_string(),
        ];
        assert!(is_valid_order(&indices));
    }

    #[test]
    fn test_invalid_ordering() {
        let indices = vec!["b0".to_string(), "a0".to_string()];
        assert!(!is_valid_order(&indices));
    }

    #[test]
    fn test_generate_sequence() {
        let indices = generate_sequence(5);
        assert_eq!(indices.len(), 5);
        assert!(is_valid_order(&indices));
    }

    #[test]
    fn test_validation() {
        assert!(validate_index("a0").is_ok());
        assert!(validate_index("Z9").is_ok());
        assert!(validate_index("").is_err());
        assert!(validate_index("@").is_err());
    }

    #[test]
    fn test_complex_between() {
        // Test multiple levels of between operations
        let mut indices = vec!["a0".to_string(), "z9".to_string()];

        // Insert between existing indices multiple times
        for _ in 0..5 {
            let mid = between(&indices[0], &indices[1]).unwrap();
            indices.insert(1, mid);
        }

        assert!(is_valid_order(&indices));
        assert_eq!(indices.len(), 7);
    }
}
