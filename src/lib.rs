// https://github.com/taiki-e/cargo-llvm-cov#exclude-code-from-coverage
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

// TODO: implement core logic and public API
pub fn add(x: u64, y: u64) -> u64 {
    x + y
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

    #[test]
    fn example() {
        assert_eq!(add(1, 1), 2);
    }
}
