const FEE_RATE: f64 = 0.25;
const EXPONENT: f64 = 2.0;

/// Polymarket fee rate as a function of price.
/// Formula: feeRate × (p × (1 - p))^exponent
/// Returns the fee as a decimal fraction (e.g. 0.015625 for 1.56%).
pub fn polymarket_fee_pct(price: f64) -> f64 {
    FEE_RATE * (price * (1.0 - price)).powf(EXPONENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round a decimal fraction to a percentage with 2 decimal places.
    fn to_pct_rounded(fee: f64) -> f64 {
        (fee * 10000.0).round() / 100.0
    }

    #[test]
    fn fee_at_050_is_1_56_pct() {
        let fee = polymarket_fee_pct(0.50);
        assert_eq!(to_pct_rounded(fee), 1.56);
    }

    #[test]
    fn fee_at_085_is_0_41_pct() {
        let fee = polymarket_fee_pct(0.85);
        assert_eq!(to_pct_rounded(fee), 0.41);
    }

    #[test]
    fn fee_at_015_is_0_41_pct() {
        let fee = polymarket_fee_pct(0.15);
        assert_eq!(to_pct_rounded(fee), 0.41);
    }
}
