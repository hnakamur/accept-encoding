#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Copy, Clone)]
pub struct QValue {
    millis: u16,
}

#[derive(Debug, PartialEq)]
pub struct InvaliQValueError;

pub(crate) const Q_VALUE_FRAC_MAX_DIGITS: u32 = 3;

impl QValue {
    pub(crate) fn from_millis(millis: u16) -> Result<Self, InvaliQValueError> {
        if millis <= 10u16.pow(Q_VALUE_FRAC_MAX_DIGITS) {
            Ok(Self { millis })
        } else {
            Err(InvaliQValueError)
        }
    }
}

impl TryFrom<&str> for QValue {
    type Error = InvaliQValueError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        const MAX_LEN: usize = 2 + Q_VALUE_FRAC_MAX_DIGITS as usize;
        let v = s.as_bytes();
        if !v.is_empty() {
            match v[0] {
                b'0' => {
                    if (v.len() > 1 && v[1] != b'.') || v.len() > MAX_LEN {
                        return Err(InvaliQValueError);
                    }
                    let mut millis: u16 = 0;
                    if v.len() > 2 {
                        for b in &v[2..] {
                            match *b {
                                b'0'..=b'9' => {
                                    millis *= 10;
                                    millis += (*b - b'0') as u16;
                                }
                                _ => return Err(InvaliQValueError),
                            }
                        }
                        for _ in 0..MAX_LEN - v.len() {
                            millis *= 10;
                        }
                    }
                    return Ok(Self { millis });
                }
                b'1' => {
                    if (v.len() > 1 && v[1] != b'.') || v.len() > MAX_LEN {
                        return Err(InvaliQValueError);
                    }
                    if v.len() > 2 {
                        for b in &v[2..] {
                            if *b != b'0' {
                                return Err(InvaliQValueError);
                            }
                        }
                    }
                    return Ok(Self { millis: 1000 });
                }
                _ => {}
            }
        }
        Err(InvaliQValueError)
    }
}

impl TryFrom<f64> for QValue {
    type Error = InvaliQValueError;
    #[inline]
    fn try_from(v: f64) -> Result<Self, Self::Error> {
        if v.is_nan() || !(0.0..=1.0).contains(&v) {
            Err(InvaliQValueError)
        } else {
            QValue::from_millis((v * 10u16.pow(Q_VALUE_FRAC_MAX_DIGITS) as f64) as u16)
        }
    }
}

impl From<QValue> for f64 {
    fn from(source: QValue) -> f64 {
        source.millis as f64 / 10_u32.pow(Q_VALUE_FRAC_MAX_DIGITS) as f64
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_qvalue_from_byte_slice() {
        assert_eq!(Ok(QValue { millis: 0 }), QValue::try_from("0"));
        assert_eq!(Ok(QValue { millis: 0 }), QValue::try_from("0."));
        assert_eq!(Ok(QValue { millis: 100 }), QValue::try_from("0.1"));
        assert_eq!(Ok(QValue { millis: 120 }), QValue::try_from("0.12"));
        assert_eq!(Ok(QValue { millis: 123 }), QValue::try_from("0.123"));
        assert_eq!(Err(InvaliQValueError), QValue::try_from("0.1235"));

        assert_eq!(Ok(QValue { millis: 1000 }), QValue::try_from("1"));
        assert_eq!(Ok(QValue { millis: 1000 }), QValue::try_from("1."));
        assert_eq!(Ok(QValue { millis: 1000 }), QValue::try_from("1.0"));
        assert_eq!(Ok(QValue { millis: 1000 }), QValue::try_from("1.00"));
        assert_eq!(Ok(QValue { millis: 1000 }), QValue::try_from("1.000"));
        assert_eq!(Err(InvaliQValueError), QValue::try_from("1.0000"));
        assert_eq!(Err(InvaliQValueError), QValue::try_from("1.1"));

        assert_eq!(Err(InvaliQValueError), QValue::try_from("-0"));
    }
}
