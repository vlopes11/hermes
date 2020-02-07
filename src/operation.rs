use std::io;
use std::ops::{Neg, Rem};

#[cfg(feature = "trace")]
use tracing::trace;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Operation {
    Undefined,
    Return,
    Store(u32),

    Abs,
    Acos,
    Acosh,
    Asin,
    Asinh,
    Atan,
    Atanh,
    Cbrt,
    Ceil,
    Cos,
    Cosh,
    Exp,
    Exp2,
    ExpM1,
    Floor,
    Fract,
    Ln,
    Ln1p,
    Log(f64),
    Log10,
    Log2,
    Neg,
    Pow(f64),
    Recip,
    Round,
    Signum,
    Sin,
    Sinh,
    Sqrt,
    Tan,
    Tanh,
    ToRadians,
    Trunc,

    Add(u32, u32),
    Atan2(u32, u32),
    Div(u32, u32),
    DivEuclid(u32, u32),
    Hypot(u32, u32),
    Max(u32, u32),
    Min(u32, u32),
    Mul(u32, u32),
    Rem(u32, u32),
    RemEuclid(u32, u32),
    Sub(u32, u32),
}

impl Operation {
    pub fn to_vec(self) -> Vec<u8> {
        self.into()
    }

    pub fn execute(self, x: f64) -> f64 {
        #[cfg(feature = "trace")]
        trace!("Executing single {:?}", self);

        let r = match self {
            Operation::Abs => x.abs(),
            Operation::Acos => x.acos(),
            Operation::Acosh => x.acosh(),
            Operation::Asin => x.asin(),
            Operation::Asinh => x.asinh(),
            Operation::Atan => x.atan(),
            Operation::Atanh => x.atanh(),
            Operation::Cbrt => x.cbrt(),
            Operation::Ceil => x.ceil(),
            Operation::Cos => x.cos(),
            Operation::Cosh => x.cosh(),
            Operation::Exp => x.exp(),
            Operation::Exp2 => x.exp2(),
            Operation::ExpM1 => x.exp_m1(),
            Operation::Floor => x.floor(),
            Operation::Fract => x.fract(),
            Operation::Ln => x.ln(),
            Operation::Ln1p => x.ln_1p(),
            Operation::Log(f) => x.log(f),
            Operation::Log10 => x.log10(),
            Operation::Log2 => x.log2(),
            Operation::Neg => x.neg(),
            Operation::Pow(f) => x.powf(f),
            Operation::Recip => x.recip(),
            Operation::Round => x.round(),
            Operation::Signum => x.signum(),
            Operation::Sin => x.sin(),
            Operation::Sinh => x.sinh(),
            Operation::Sqrt => x.sqrt(),
            Operation::Tan => x.tan(),
            Operation::Tanh => x.tanh(),
            Operation::ToRadians => x.to_radians(),
            Operation::Trunc => x.trunc(),
            _ => 0.00f64,
        };

        #[cfg(feature = "trace")]
        trace!("Result single {:?} -> {}", self, r);
        r
    }

    /// # Panics
    ///
    /// Will panic if the number of provided terms is not equal to the expected size
    pub fn execute_multiple(self, terms: &[f64]) -> f64 {
        #[cfg(feature = "trace")]
        trace!("Executing multiple {:?}({:?})", self, terms);

        let r = match self {
            Operation::Add(_, l) => terms[1..l as usize].iter().fold(terms[0], |x, y| x + y),
            Operation::Atan2(_, l) => terms[1..l as usize]
                .iter()
                .fold(terms[0], |x, y| x.atan2(*y)),
            Operation::Div(_, l) => terms[1..l as usize].iter().fold(terms[0], |x, y| x / y),
            Operation::DivEuclid(_, l) => terms[1..l as usize]
                .iter()
                .fold(terms[0], |x, y| x.div_euclid(*y)),
            Operation::Hypot(_, l) => terms[1..l as usize]
                .iter()
                .fold(terms[0], |x, y| x.hypot(*y)),
            Operation::Max(_, l) => terms[1..l as usize].iter().fold(terms[0], |x, y| x.max(*y)),
            Operation::Min(_, l) => terms[1..l as usize].iter().fold(terms[0], |x, y| x.min(*y)),
            Operation::Mul(_, l) => terms[1..l as usize].iter().fold(terms[0], |x, y| x * y),
            Operation::Rem(_, l) => terms[1..l as usize].iter().fold(terms[0], |x, y| x.rem(*y)),
            Operation::RemEuclid(_, l) => terms[1..l as usize]
                .iter()
                .fold(terms[0], |x, y| x.rem_euclid(*y)),
            Operation::Sub(_, l) => terms[1..l as usize].iter().fold(terms[0], |x, y| x - y),
            _ => 0.00f64,
        };

        #[cfg(feature = "trace")]
        trace!("Result multiple {:?}({:?}) -> {}", self, terms, r);
        r
    }

    pub fn fetch_size(&self) -> (u32, u32) {
        match self {
            Operation::Add(l, s) => (*l, *s),
            Operation::Atan2(l, s) => (*l, *s),
            Operation::Div(l, s) => (*l, *s),
            Operation::DivEuclid(l, s) => (*l, *s),
            Operation::Hypot(l, s) => (*l, *s),
            Operation::Max(l, s) => (*l, *s),
            Operation::Min(l, s) => (*l, *s),
            Operation::Mul(l, s) => (*l, *s),
            Operation::Rem(l, s) => (*l, *s),
            Operation::RemEuclid(l, s) => (*l, *s),
            Operation::Sub(l, s) => (*l, *s),
            _ => (0, 0),
        }
    }

    pub fn byte_code(&self) -> u8 {
        match self {
            Operation::Undefined => 0x00,
            Operation::Return => 0x01,
            Operation::Store(_) => 0x02,
            Operation::Abs => 0x03,
            Operation::Acos => 0x04,
            Operation::Acosh => 0x05,
            Operation::Asin => 0x07,
            Operation::Asinh => 0x08,
            Operation::Atan => 0x09,
            Operation::Atanh => 0x0A,
            Operation::Cbrt => 0x0B,
            Operation::Ceil => 0x0C,
            Operation::Cos => 0x0D,
            Operation::Cosh => 0x0E,
            Operation::Exp => 0x0F,
            Operation::Exp2 => 0x10,
            Operation::ExpM1 => 0x11,
            Operation::Floor => 0x12,
            Operation::Fract => 0x13,
            Operation::Ln => 0x14,
            Operation::Ln1p => 0x15,
            Operation::Log(_) => 0x27,
            Operation::Log10 => 0x16,
            Operation::Log2 => 0x17,
            Operation::Neg => 0x2C,
            Operation::Pow(_) => 0x2D,
            Operation::Recip => 0x18,
            Operation::Round => 0x19,
            Operation::Signum => 0x1A,
            Operation::Sin => 0x1B,
            Operation::Sinh => 0x1D,
            Operation::Sqrt => 0x1E,
            Operation::Tan => 0x1F,
            Operation::Tanh => 0x20,
            Operation::ToRadians => 0x21,
            Operation::Trunc => 0x22,
            Operation::Add(_, _) => 0x06,
            Operation::Atan2(_, _) => 0x23,
            Operation::Div(_, _) => 0x24,
            Operation::DivEuclid(_, _) => 0x25,
            Operation::Hypot(_, _) => 0x26,
            Operation::Max(_, _) => 0x28,
            Operation::Min(_, _) => 0x29,
            Operation::Mul(_, _) => 0x2A,
            Operation::Rem(_, _) => 0x2E,
            Operation::RemEuclid(_, _) => 0x2F,
            Operation::Sub(_, _) => 0x30,
        }
    }
}

impl From<[u8; 9]> for Operation {
    fn from(bytes: [u8; 9]) -> Self {
        let c = bytes[0];

        let mut a = [0x00u8; 4];
        let mut b = [0x00u8; 4];

        a.copy_from_slice(&bytes[1..5]);
        b.copy_from_slice(&bytes[5..9]);

        let a = u32::from_le_bytes(a);
        let b = u32::from_le_bytes(b);

        match c {
            0x01 => Operation::Return,
            0x02 => Operation::Store(a),
            0x03 => Operation::Abs,
            0x04 => Operation::Acos,
            0x05 => Operation::Acosh,
            0x07 => Operation::Asin,
            0x08 => Operation::Asinh,
            0x09 => Operation::Atan,
            0x0A => Operation::Atanh,
            0x0B => Operation::Cbrt,
            0x0C => Operation::Ceil,
            0x0D => Operation::Cos,
            0x0E => Operation::Cosh,
            0x0F => Operation::Exp,
            0x10 => Operation::Exp2,
            0x11 => Operation::ExpM1,
            0x12 => Operation::Floor,
            0x13 => Operation::Fract,
            0x14 => Operation::Ln,
            0x15 => Operation::Ln1p,
            0x27 => Operation::Log(ab_to_f64(a, b)),
            0x16 => Operation::Log10,
            0x17 => Operation::Log2,
            0x2C => Operation::Neg,
            0x2D => Operation::Pow(ab_to_f64(a, b)),
            0x18 => Operation::Recip,
            0x19 => Operation::Round,
            0x1A => Operation::Signum,
            0x1B => Operation::Sin,
            0x1D => Operation::Sinh,
            0x1E => Operation::Sqrt,
            0x1F => Operation::Tan,
            0x20 => Operation::Tanh,
            0x21 => Operation::ToRadians,
            0x22 => Operation::Trunc,
            0x06 => Operation::Add(a, b),
            0x23 => Operation::Atan2(a, b),
            0x24 => Operation::Div(a, b),
            0x25 => Operation::DivEuclid(a, b),
            0x26 => Operation::Hypot(a, b),
            0x28 => Operation::Max(a, b),
            0x29 => Operation::Min(a, b),
            0x2A => Operation::Mul(a, b),
            0x2E => Operation::Rem(a, b),
            0x2F => Operation::RemEuclid(a, b),
            0x30 => Operation::Sub(a, b),

            _ => Operation::Undefined,
        }
    }
}

impl Into<[u8; 9]> for Operation {
    fn into(self) -> [u8; 9] {
        let (c, a, b) = match self {
            Operation::Undefined => (0x00, 0, 0),
            Operation::Return => (0x01, 0, 0),
            Operation::Store(a) => (0x02, a, 0),
            Operation::Abs => (0x03, 0, 0),
            Operation::Acos => (0x04, 0, 0),
            Operation::Acosh => (0x05, 0, 0),
            Operation::Asin => (0x07, 0, 0),
            Operation::Asinh => (0x08, 0, 0),
            Operation::Atan => (0x09, 0, 0),
            Operation::Atanh => (0x0A, 0, 0),
            Operation::Cbrt => (0x0B, 0, 0),
            Operation::Ceil => (0x0C, 0, 0),
            Operation::Cos => (0x0D, 0, 0),
            Operation::Cosh => (0x0E, 0, 0),
            Operation::Exp => (0x0F, 0, 0),
            Operation::Exp2 => (0x10, 0, 0),
            Operation::ExpM1 => (0x11, 0, 0),
            Operation::Floor => (0x12, 0, 0),
            Operation::Fract => (0x13, 0, 0),
            Operation::Ln => (0x14, 0, 0),
            Operation::Ln1p => (0x15, 0, 0),
            Operation::Log(f) => f64_to_cab(0x27, f),
            Operation::Log10 => (0x16, 0, 0),
            Operation::Log2 => (0x17, 0, 0),
            Operation::Neg => (0x2C, 0, 0),
            Operation::Pow(f) => f64_to_cab(0x2D, f),
            Operation::Recip => (0x18, 0, 0),
            Operation::Round => (0x19, 0, 0),
            Operation::Signum => (0x1A, 0, 0),
            Operation::Sin => (0x1B, 0, 0),
            Operation::Sinh => (0x1D, 0, 0),
            Operation::Sqrt => (0x1E, 0, 0),
            Operation::Tan => (0x1F, 0, 0),
            Operation::Tanh => (0x20, 0, 0),
            Operation::ToRadians => (0x21, 0, 0),
            Operation::Trunc => (0x22, 0, 0),
            Operation::Add(a, b) => (0x06, a, b),
            Operation::Atan2(a, b) => (0x23, a, b),
            Operation::Div(a, b) => (0x24, a, b),
            Operation::DivEuclid(a, b) => (0x25, a, b),
            Operation::Hypot(a, b) => (0x26, a, b),
            Operation::Max(a, b) => (0x28, a, b),
            Operation::Min(a, b) => (0x29, a, b),
            Operation::Mul(a, b) => (0x2A, a, b),
            Operation::Rem(a, b) => (0x2E, a, b),
            Operation::RemEuclid(a, b) => (0x2F, a, b),
            Operation::Sub(a, b) => (0x30, a, b),
        };

        let mut bytes = [0x00; 9];

        bytes[0] = c;
        (&mut bytes[1..5]).copy_from_slice(&a.to_le_bytes()[..]);
        (&mut bytes[5..9]).copy_from_slice(&b.to_le_bytes()[..]);

        bytes
    }
}

pub struct OperationIterator<R: io::Read> {
    reader: R,
}

impl<R: io::Read> OperationIterator<R> {
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<R: io::Read> Iterator for OperationIterator<R> {
    type Item = Operation;

    fn next(&mut self) -> Option<Self::Item> {
        let mut bytes = [0x00u8; 9];

        if self.reader.read_exact(&mut bytes).is_err() {
            return None;
        }

        Some(bytes.into())
    }
}

impl Into<Vec<u8>> for Operation {
    fn into(self) -> Vec<u8> {
        let bytes: [u8; 9] = self.into();
        bytes.to_vec()
    }
}

impl From<Vec<u8>> for Operation {
    fn from(bytes: Vec<u8>) -> Self {
        if bytes.len() != 9 {
            Operation::Undefined
        } else {
            let mut op = [0x00u8; 9];
            op.copy_from_slice(bytes.as_slice());
            op.into()
        }
    }
}

fn ab_to_f64(a: u32, b: u32) -> f64 {
    let mut f = [0x00u8; 8];

    (&mut f[0..4]).copy_from_slice(&a.to_le_bytes()[..]);
    (&mut f[4..8]).copy_from_slice(&b.to_le_bytes()[..]);

    f64::from_le_bytes(f)
}

fn f64_to_cab(c: u8, f: f64) -> (u8, u32, u32) {
    let f = f.to_le_bytes();

    let mut a = [0x00u8; 4];
    let mut b = [0x00u8; 4];

    a.copy_from_slice(&f[0..4]);
    b.copy_from_slice(&f[4..8]);

    let a = u32::from_le_bytes(a);
    let b = u32::from_le_bytes(b);

    (c, a, b)
}
