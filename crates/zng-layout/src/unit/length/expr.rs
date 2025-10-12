use std::fmt;

use zng_unit::{ByteLength, ByteUnits as _, Factor, Px};
use zng_var::animation::Transitionable as _;

use crate::{
    context::LayoutMask,
    unit::{Layout1d, LayoutAxis, Length, ParseCompositeError},
};

/// Represents an unresolved [`Length`] expression.
#[derive(Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[non_exhaustive]
pub enum LengthExpr {
    /// Sums the both layout length.
    Add(Length, Length),
    /// Subtracts the first layout length from the second.
    Sub(Length, Length),
    /// Multiplies the layout length by the factor.
    Mul(Length, Factor),
    /// Divide the layout length by the factor.
    Div(Length, Factor),
    /// Maximum layout length.
    Max(Length, Length),
    /// Minimum layout length.
    Min(Length, Length),
    /// Computes the absolute layout length.
    Abs(Length),
    /// Negate the layout length.
    Neg(Length),
    /// Linear interpolate between lengths by factor.
    Lerp(Length, Length, Factor),
}
impl LengthExpr {
    /// Gets the total memory allocated by this length expression.
    ///
    /// This includes the sum of all nested [`Length::Expr`] heap memory.
    pub fn memory_used(&self) -> ByteLength {
        use LengthExpr::*;
        std::mem::size_of::<LengthExpr>().bytes()
            + match self {
                Add(a, b) => a.heap_memory_used() + b.heap_memory_used(),
                Sub(a, b) => a.heap_memory_used() + b.heap_memory_used(),
                Mul(a, _) => a.heap_memory_used(),
                Div(a, _) => a.heap_memory_used(),
                Max(a, b) => a.heap_memory_used() + b.heap_memory_used(),
                Min(a, b) => a.heap_memory_used() + b.heap_memory_used(),
                Abs(a) => a.heap_memory_used(),
                Neg(a) => a.heap_memory_used(),
                Lerp(a, b, _) => a.heap_memory_used() + b.heap_memory_used(),
            }
    }

    /// Convert to [`Length::Expr`], logs warning for memory use above 1kB, logs error for use > 20kB and collapses to [`Length::zero`].
    ///
    /// Every length expression created using the [`std::ops`] uses this method to check the constructed expression. Some operations
    /// like iterator fold can cause an *expression explosion* where two lengths of different units that cannot
    /// be evaluated immediately start an expression that subsequently is wrapped in a new expression for each operation done on it.
    pub fn to_length_checked(self) -> Length {
        let bytes = self.memory_used();
        if bytes > 20.kibibytes() {
            tracing::error!(target: "to_length_checked", "length alloc > 20kB, replaced with zero");
            return Length::zero();
        }
        Length::Expr(Box::new(self))
    }

    /// If contains a [`Length::Default`] value.
    pub fn has_default(&self) -> bool {
        match self {
            LengthExpr::Add(a, b) | LengthExpr::Sub(a, b) | LengthExpr::Max(a, b) | LengthExpr::Min(a, b) | LengthExpr::Lerp(a, b, _) => {
                a.has_default() || b.has_default()
            }
            LengthExpr::Mul(a, _) | LengthExpr::Div(a, _) | LengthExpr::Abs(a) | LengthExpr::Neg(a) => a.has_default(),
        }
    }

    /// Replace all [`Length::Default`] values with `overwrite`.
    pub fn replace_default(&mut self, overwrite: &Length) {
        match self {
            LengthExpr::Add(a, b) | LengthExpr::Sub(a, b) | LengthExpr::Max(a, b) | LengthExpr::Min(a, b) | LengthExpr::Lerp(a, b, _) => {
                a.replace_default(overwrite);
                b.replace_default(overwrite);
            }
            LengthExpr::Mul(a, _) | LengthExpr::Div(a, _) | LengthExpr::Abs(a) | LengthExpr::Neg(a) => a.replace_default(overwrite),
        }
    }

    /// Convert [`PxF32`] to [`Px`] and [`DipF32`] to [`Dip`].
    ///
    /// [`PxF32`]: Length::PxF32
    /// [`Px`]: Length::Px
    /// [`DipF32`]: Length::DipF32
    /// [`Dip`]: Length::Dip
    pub fn round_exact(&mut self) {
        match self {
            LengthExpr::Add(a, b) | LengthExpr::Sub(a, b) | LengthExpr::Max(a, b) | LengthExpr::Min(a, b) | LengthExpr::Lerp(a, b, _) => {
                a.round_exact();
                b.round_exact();
            }
            LengthExpr::Mul(a, _) | LengthExpr::Div(a, _) | LengthExpr::Abs(a) | LengthExpr::Neg(a) => a.round_exact(),
        }
    }
}
impl Layout1d for LengthExpr {
    fn layout_dft(&self, axis: LayoutAxis, default: Px) -> Px {
        let l = self.layout_f32_dft(axis, default.0 as f32);
        Px(l.round() as i32)
    }

    fn layout_f32_dft(&self, axis: LayoutAxis, default: f32) -> f32 {
        use LengthExpr::*;
        match self {
            Add(a, b) => a.layout_f32_dft(axis, default) + b.layout_f32_dft(axis, default),
            Sub(a, b) => a.layout_f32_dft(axis, default) - b.layout_f32_dft(axis, default),
            Mul(l, s) => l.layout_f32_dft(axis, default) * s.0,
            Div(l, s) => l.layout_f32_dft(axis, default) / s.0,
            Max(a, b) => {
                let a = a.layout_f32_dft(axis, default);
                let b = b.layout_f32_dft(axis, default);
                a.max(b)
            }
            Min(a, b) => {
                let a = a.layout_f32_dft(axis, default);
                let b = b.layout_f32_dft(axis, default);
                a.min(b)
            }
            Abs(e) => e.layout_f32_dft(axis, default).abs(),
            Neg(e) => -e.layout_f32_dft(axis, default),
            Lerp(a, b, f) => a.layout_f32_dft(axis, default).lerp(&b.layout_f32_dft(axis, default), *f),
        }
    }

    fn affect_mask(&self) -> LayoutMask {
        use LengthExpr::*;
        match self {
            Add(a, b) => a.affect_mask() | b.affect_mask(),
            Sub(a, b) => a.affect_mask() | b.affect_mask(),
            Mul(a, _) => a.affect_mask(),
            Div(a, _) => a.affect_mask(),
            Max(a, b) => a.affect_mask() | b.affect_mask(),
            Min(a, b) => a.affect_mask() | b.affect_mask(),
            Abs(a) => a.affect_mask(),
            Neg(a) => a.affect_mask(),
            Lerp(a, b, _) => a.affect_mask() | b.affect_mask(),
        }
    }
}
impl fmt::Debug for LengthExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LengthExpr::*;
        if f.alternate() {
            match self {
                Add(a, b) => f.debug_tuple("LengthExpr::Add").field(a).field(b).finish(),
                Sub(a, b) => f.debug_tuple("LengthExpr::Sub").field(a).field(b).finish(),
                Mul(l, s) => f.debug_tuple("LengthExpr::Mul").field(l).field(s).finish(),
                Div(l, s) => f.debug_tuple("LengthExpr::Div").field(l).field(s).finish(),
                Max(a, b) => f.debug_tuple("LengthExpr::Max").field(a).field(b).finish(),
                Min(a, b) => f.debug_tuple("LengthExpr::Min").field(a).field(b).finish(),
                Abs(e) => f.debug_tuple("LengthExpr::Abs").field(e).finish(),
                Neg(e) => f.debug_tuple("LengthExpr::Neg").field(e).finish(),
                Lerp(a, b, n) => f.debug_tuple("LengthExpr::Lerp").field(a).field(b).field(n).finish(),
            }
        } else {
            match self {
                Add(a, b) => write!(f, "({a:?} + {b:?})"),
                Sub(a, b) => write!(f, "({a:?} - {b:?})"),
                Mul(l, s) => write!(f, "({l:?} * {:?}.pct())", s.0 * 100.0),
                Div(l, s) => write!(f, "({l:?} / {:?}.pct())", s.0 * 100.0),
                Max(a, b) => write!(f, "max({a:?}, {b:?})"),
                Min(a, b) => write!(f, "min({a:?}, {b:?})"),
                Abs(e) => write!(f, "abs({e:?})"),
                Neg(e) => write!(f, "-({e:?})"),
                Lerp(a, b, n) => write!(f, "lerp({a:?}, {b:?}, {n:?})"),
            }
        }
    }
}
impl fmt::Display for LengthExpr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use LengthExpr::*;
        match self {
            Add(a, b) => write!(f, "({a} + {b})"),
            Sub(a, b) => write!(f, "({a} - {b})"),
            Mul(l, s) => write!(f, "({l} * {}%)", s.0 * 100.0),
            Div(l, s) => write!(f, "({l} / {}%)", s.0 * 100.0),
            Max(a, b) => write!(f, "max({a}, {b})"),
            Min(a, b) => write!(f, "min({a}, {b})"),
            Abs(e) => write!(f, "abs({e})"),
            Neg(e) => write!(f, "-({e})"),
            Lerp(a, b, n) => write!(f, "lerp({a}, {b}, {n})"),
        }
    }
}
impl std::str::FromStr for LengthExpr {
    type Err = ParseCompositeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let expr = Parser::new(s).parse()?;
        match Length::try_from(expr)? {
            Length::Expr(expr) => Ok(*expr),
            _ => Err(ParseCompositeError::MissingComponent),
        }
    }
}

impl<'a> TryFrom<Expr<'a>> for Length {
    type Error = ParseCompositeError;

    fn try_from(value: Expr) -> Result<Self, Self::Error> {
        match value {
            Expr::Value(l) => l.parse(),
            Expr::UnaryOp { op, rhs } => match op {
                '-' => Ok(LengthExpr::Neg(Length::try_from(*rhs)?).into()),
                '+' => Length::try_from(*rhs),
                _ => Err(ParseCompositeError::UnknownFormat),
            },
            Expr::BinaryOp { op, lhs, rhs } => match op {
                '+' => Ok(LengthExpr::Add(Length::try_from(*lhs)?, Length::try_from(*rhs)?).into()),
                '-' => Ok(LengthExpr::Sub(Length::try_from(*lhs)?, Length::try_from(*rhs)?).into()),
                '*' => Ok(LengthExpr::Mul(Length::try_from(*lhs)?, try_into_scale(*rhs)?).into()),
                '/' => Ok(LengthExpr::Div(Length::try_from(*lhs)?, try_into_scale(*rhs)?).into()),
                _ => Err(ParseCompositeError::UnknownFormat),
            },
            Expr::Call { name, mut args } => match name {
                "max" => {
                    let [a, b] = try_args(args)?;
                    Ok(LengthExpr::Max(a, b).into())
                }
                "min" => {
                    let [a, b] = try_args(args)?;
                    Ok(LengthExpr::Min(a, b).into())
                }
                "abs" => {
                    let [a] = try_args(args)?;
                    Ok(LengthExpr::Abs(a).into())
                }
                "lerp" => {
                    let s = args.pop().ok_or(ParseCompositeError::MissingComponent)?;
                    let [a, b] = try_args(args)?;
                    let s = try_into_scale(s)?;
                    Ok(LengthExpr::Lerp(a, b, s).into())
                }
                _ => Err(ParseCompositeError::UnknownFormat),
            },
        }
    }
}
fn try_into_scale(rhs: Expr) -> Result<Factor, ParseCompositeError> {
    if let Length::Factor(f) = Length::try_from(rhs)? {
        Ok(f)
    } else {
        Err(ParseCompositeError::UnknownFormat)
    }
}
fn try_args<const N: usize>(args: Vec<Expr>) -> Result<[Length; N], ParseCompositeError> {
    match args.len().cmp(&N) {
        std::cmp::Ordering::Less => Err(ParseCompositeError::MissingComponent),
        std::cmp::Ordering::Equal => Ok(args
            .into_iter()
            .map(Length::try_from)
            .collect::<Result<Vec<Length>, ParseCompositeError>>()?
            .try_into()
            .unwrap()),
        std::cmp::Ordering::Greater => Err(ParseCompositeError::ExtraComponent),
    }
}

/// Basic string representation of `lengthExpr`, without validating functions and Length values.
#[derive(Debug, PartialEq)]
enum Expr<'a> {
    #[allow(unused)]
    Value(&'a str),
    UnaryOp {
        op: char,
        rhs: Box<Expr<'a>>,
    },
    BinaryOp {
        op: char,
        lhs: Box<Expr<'a>>,
        rhs: Box<Expr<'a>>,
    },
    Call {
        name: &'a str,
        args: Vec<Expr<'a>>,
    },
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    len: usize,
}
impl<'a> Parser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            len: input.len(),
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        if self.pos >= self.len {
            return None;
        }
        let ch = self.peek_char()?;
        self.pos += ch.len_utf8();
        Some(ch)
    }

    fn consume_whitespace(&mut self) {
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() {
                self.next_char();
            } else {
                break;
            }
        }
    }

    fn starts_with_nonop(&self, ch: char) -> bool {
        !ch.is_whitespace() && !matches!(ch, '+' | '-' | '*' | '/' | '(' | ')' | ',')
    }

    fn parse_value_token(&mut self) -> Result<&'a str, ParseCompositeError> {
        self.consume_whitespace();
        let start = self.pos;
        while let Some(ch) = self.peek_char() {
            if self.starts_with_nonop(ch) {
                self.next_char();
            } else {
                break;
            }
        }
        let s = &self.input[start..self.pos];
        if s.is_empty() {
            Err(ParseCompositeError::MissingComponent)
        } else {
            Ok(s)
        }
    }

    pub fn parse(&mut self) -> Result<Expr<'a>, ParseCompositeError> {
        self.consume_whitespace();
        let expr = self.parse_expr_bp(0)?;
        self.consume_whitespace();
        if self.pos < self.len {
            Err(ParseCompositeError::ExtraComponent)
        } else {
            Ok(expr)
        }
    }

    fn infix_binding_power(op: char) -> Option<(u32, u32)> {
        match op {
            '+' | '-' => Some((10, 11)), // low precedence
            '*' | '/' => Some((20, 21)), // higher precedence
            _ => None,
        }
    }

    fn parse_expr_bp(&mut self, min_bp: u32) -> Result<Expr<'a>, ParseCompositeError> {
        self.consume_whitespace();

        // --- prefix / primary ---
        let mut lhs = match self.peek_char() {
            Some('-') => {
                // unary -
                self.next_char();
                let rhs = self.parse_expr_bp(100)?; // high precedence for unary
                Expr::UnaryOp {
                    op: '-',
                    rhs: Box::new(rhs),
                }
            }
            Some('(') => {
                // parenthesized expression
                self.next_char(); // consume '('
                let inner = self.parse_expr_bp(0)?;
                self.consume_whitespace();
                match self.next_char() {
                    Some(')') => inner,
                    _ => return Err(ParseCompositeError::MissingComponent),
                }
            }
            Some(ch) if self.starts_with_nonop(ch) => {
                // value token or function call
                let token = self.parse_value_token()?;
                // check if function call: next non-space char is '('
                self.consume_whitespace();
                if let Some('(') = self.peek_char() {
                    // function call: name(token) (must have at least one arg)
                    let name = token;
                    self.next_char(); // consume '('
                    let mut args = Vec::new();
                    self.consume_whitespace();
                    if let Some(')') = self.peek_char() {
                        return Err(ParseCompositeError::MissingComponent);
                    }
                    // parse first arg
                    loop {
                        self.consume_whitespace();
                        let arg = self.parse_expr_bp(0)?;
                        args.push(arg);
                        self.consume_whitespace();
                        match self.peek_char() {
                            Some(',') => {
                                self.next_char();
                                continue;
                            }
                            Some(')') => {
                                self.next_char();
                                break;
                            }
                            Some(_) => return Err(ParseCompositeError::ExtraComponent),
                            None => return Err(ParseCompositeError::MissingComponent),
                        }
                    }
                    Expr::Call { name, args }
                } else {
                    Expr::Value(token)
                }
            }
            Some(_) => return Err(ParseCompositeError::ExtraComponent),
            None => return Err(ParseCompositeError::MissingComponent),
        };

        // --- infix loop: while there's an operator with precedence >= min_bp ---
        loop {
            self.consume_whitespace();
            let op = match self.peek_char() {
                Some(c) if matches!(c, '+' | '-' | '*' | '/') => c,
                _ => break,
            };

            if let Some((l_bp, r_bp)) = Self::infix_binding_power(op) {
                if l_bp < min_bp {
                    break;
                }
                // consume operator
                self.next_char();
                // parse rhs with r_bp
                let rhs = self.parse_expr_bp(r_bp)?;
                lhs = Expr::BinaryOp {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                };
            } else {
                break;
            }
        }

        Ok(lhs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(s: &str) -> Expr<'_> {
        let mut p = Parser::new(s);
        p.parse().unwrap()
    }

    #[test]
    fn test_values() {
        assert_eq!(parse_ok("default"), Expr::Value("default"));
        assert_eq!(parse_ok("3.14"), Expr::Value("3.14"));
        assert_eq!(parse_ok("abc.def"), Expr::Value("abc.def"));
    }

    #[test]
    fn test_unary() {
        assert_eq!(
            parse_ok("-x"),
            Expr::UnaryOp {
                op: '-',
                rhs: Box::new(Expr::Value("x"))
            }
        );
        assert_eq!(
            parse_ok("--3"),
            Expr::UnaryOp {
                op: '-',
                rhs: Box::new(Expr::UnaryOp {
                    op: '-',
                    rhs: Box::new(Expr::Value("3"))
                })
            }
        );
    }

    #[test]
    fn test_binary_prec() {
        // 1 + 2 * 3 => 1 + (2 * 3)
        let e = parse_ok("1 + 2 * 3");
        assert_eq!(
            e,
            Expr::BinaryOp {
                op: '+',
                lhs: Box::new(Expr::Value("1")),
                rhs: Box::new(Expr::BinaryOp {
                    op: '*',
                    lhs: Box::new(Expr::Value("2")),
                    rhs: Box::new(Expr::Value("3")),
                })
            }
        );

        // (1 + 2) * 3
        let e = parse_ok("(1 + 2) * 3");
        assert_eq!(
            e,
            Expr::BinaryOp {
                op: '*',
                lhs: Box::new(Expr::BinaryOp {
                    op: '+',
                    lhs: Box::new(Expr::Value("1")),
                    rhs: Box::new(Expr::Value("2")),
                }),
                rhs: Box::new(Expr::Value("3"))
            }
        );
    }

    #[test]
    fn test_call() {
        let e = parse_ok("f(a, b + 2, -3)");
        assert_eq!(
            e,
            Expr::Call {
                name: "f",
                args: vec![
                    Expr::Value("a"),
                    Expr::BinaryOp {
                        op: '+',
                        lhs: Box::new(Expr::Value("b")),
                        rhs: Box::new(Expr::Value("2")),
                    },
                    Expr::UnaryOp {
                        op: '-',
                        rhs: Box::new(Expr::Value("3"))
                    },
                ],
            }
        );
    }
}
