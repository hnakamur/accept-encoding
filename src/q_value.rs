#[derive(Debug, PartialOrd, Ord, PartialEq, Eq, Copy, Clone)]
pub struct QValue {
    millis: u16,
}

#[derive(Debug)]
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
