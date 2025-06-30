/// [`f32`] equality used in floating-point units.
///
/// * [`NaN`](f32::is_nan) values are equal.
/// * [`INFINITY`](f32::INFINITY) values are equal.
/// * [`NEG_INFINITY`](f32::NEG_INFINITY) values are equal.
/// * Finite values are equal if they fall in the same *bucket* sized by `granularity`.
///
/// Note that this definition of equality is symmetric, reflexive, and transitive. This is slightly different them
/// equality defined by minimal distance *epsilon*, in some cases `abs(a - b) < granularity` can be true and not equal because they
/// are near a *bucket threshold*. In practice this does not mather given sufficient small `granularity`, it is more
/// stable due to the transitive property and enables the [`about_eq_hash`] function to always output the same hash for the same values.
pub fn about_eq(a: f32, b: f32, granularity: f32) -> bool {
    f32_about_eq_snap(a, granularity) == f32_about_eq_snap(b, granularity)
}

/// [`f32`] hash compatible with [`about_eq`] equality.
pub fn about_eq_hash<H: std::hash::Hasher>(f: f32, granularity: f32, state: &mut H) {
    use std::hash::Hash;
    f32_about_eq_snap(f, granularity).hash(state);
}
fn f32_about_eq_snap(f: f32, granularity: f32) -> (u8, i64) {
    let (kind, bucket) = if f.is_nan() {
        (0u8, 0i64)
    } else if f.is_infinite() {
        let sign = if f.is_sign_positive() { 1 } else { -1 };
        (1, sign)
    } else {
        let bucket = (f / granularity).floor() as i64;
        (2, bucket)
    };
    (kind, bucket)
}

/// [`f32`] ordering compatible with [`about_eq`] equality.
pub fn about_eq_ord(a: f32, b: f32, granularity: f32) -> std::cmp::Ordering {
    if about_eq(a, b, granularity) {
        std::cmp::Ordering::Equal
    } else {
        // Fallback to the standard partial_cmp for a robust ordering.
        // This correctly handles all other cases, including comparisons with NaN.
        // partial_cmp returns None if one operand is NaN.
        // You can decide how to treat this case; here we default to Less.
        a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Less)
    }
}

/// Minimal bucket size for equality between values in around the 0.0..=1.0 scale.
pub const EQ_GRANULARITY: f32 = 0.00001;
/// Minimal bucket size for equality between values in around the 1.0..=100.0 scale.
pub const EQ_GRANULARITY_100: f32 = 0.001;

/// Minimal difference between values in around the 0.0..=1.0 scale.
#[deprecated = "use EQ_GRANULARITY"]
pub const EQ_EPSILON: f32 = 0.00001;
/// Minimal difference between values in around the 1.0..=100.0 scale.
#[deprecated = "use EQ_GRANULARITY_100"]
pub const EQ_EPSILON_100: f32 = 0.001;
