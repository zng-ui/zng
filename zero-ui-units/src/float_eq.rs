/// [`f32`] equality used in floating-point units.
///
/// * [`NaN`](f32::is_nan) values are equal.
/// * [`INFINITY`](f32::INFINITY) values are equal.
/// * [`NEG_INFINITY`](f32::NEG_INFINITY) values are equal.
/// * Finite values are equal if the difference is less than `epsilon`.
///
/// Note that this definition of equality is symmetric and reflexive, but it is **not** transitive, difference less then
/// epsilon can *accumulate* over a chain of comparisons breaking the transitive property:
///
/// ```
/// # use zero_ui_view_api::units::about_eq;
/// let e = 0.001;
/// let a = 0.0;
/// let b = a + e - 0.0001;
/// let c = b + e - 0.0001;
///
/// assert!(
///     about_eq(a, b, e) &&
///     about_eq(b, c, e) &&
///     !about_eq(a, c, e)
/// )
/// ```
///
/// See also [`about_eq_hash`].
pub fn about_eq(a: f32, b: f32, epsilon: f32) -> bool {
    if a.is_nan() {
        b.is_nan()
    } else if a.is_infinite() {
        b.is_infinite() && a.is_sign_positive() == b.is_sign_positive()
    } else {
        (a - b).abs() < epsilon
    }
}

/// [`f32`] hash compatible with [`about_eq`] equality.
pub fn about_eq_hash<H: std::hash::Hasher>(f: f32, epsilon: f32, state: &mut H) {
    let (group, f) = if f.is_nan() {
        (0u8, 0u64)
    } else if f.is_infinite() {
        (1, if f.is_sign_positive() { 1 } else { 2 })
    } else {
        let inv_epsi = if epsilon > EQ_EPSILON_100 { 100000.0 } else { 100.0 };
        (2, ((f as f64) * inv_epsi) as u64)
    };

    use std::hash::Hash;
    group.hash(state);
    f.hash(state);
}

/// [`f32`] ordering compatible with [`about_eq`] equality.
pub fn about_eq_ord(a: f32, b: f32, epsilon: f32) -> std::cmp::Ordering {
    if about_eq(a, b, epsilon) {
        std::cmp::Ordering::Equal
    } else if a > b {
        std::cmp::Ordering::Greater
    } else {
        std::cmp::Ordering::Less
    }
}

/// Minimal difference between values in around the 0.0..=1.0 scale.
pub const EQ_EPSILON: f32 = 0.00001;
/// Minimal difference between values in around the 1.0..=100.0 scale.
pub const EQ_EPSILON_100: f32 = 0.001;
