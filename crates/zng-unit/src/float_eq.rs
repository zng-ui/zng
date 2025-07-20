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
        (255u8, 0i64)
    } else if f.is_infinite() {
        if f.is_sign_positive() { (254, 0) } else { (1, 0) }
    } else {
        let bucket = (f / granularity).floor() as i64;
        (128, bucket)
    };
    (kind, bucket)
}

/// [`f32`] ordering compatible with [`about_eq`] equality.
///
/// The order is `-inf < finite < inf < NaN`.
pub fn about_eq_ord(a: f32, b: f32, granularity: f32) -> std::cmp::Ordering {
    let a = f32_about_eq_snap(a, granularity);
    let b = f32_about_eq_snap(b, granularity);
    a.cmp(&b)
}

/// Minimal bucket size for equality between values in around the 0.0..=1.0 scale.
pub const EQ_GRANULARITY: f32 = 0.00001;
/// Minimal bucket size for equality between values in around the 1.0..=100.0 scale.
pub const EQ_GRANULARITY_100: f32 = 0.001;
