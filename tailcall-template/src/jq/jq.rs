use std::ops::Deref;

use jaq_core::ValR;

use crate::jsonlike::{JsonLike, JsonObjectLike};

#[derive(Clone, PartialEq, PartialOrd)]
pub struct JsonLikeHelper<A: for<'a> JsonLike<'a>>(pub A);

impl<A> Deref for JsonLikeHelper<A> where
A: for<'a> JsonLike<'a> + Clone + PartialEq + PartialOrd {
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A> From<A> for JsonLikeHelper<A> where
A: for<'a> JsonLike<'a> + Clone + PartialEq + PartialOrd  {
    fn from(value: A) -> Self {
        Self(value)
    }
}

impl<A> jaq_core::ValT for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + Clone + PartialEq + PartialOrd,
{
    fn from_num(n: &str) -> ValR<Self> {
        match n.parse::<f64>() {
            Ok(num) => ValR::Ok(JsonLikeHelper(A::number_f64(num))),
            Err(err) => ValR::Err(jaq_core::Error::str(format!("Invalid number format: {}", err.to_string()))),
        }
    }

    fn from_map<I: IntoIterator<Item = (Self, Self)>>(iter: I) -> ValR<Self> {
        iter.into_iter().fold(ValR::Ok(Self(JsonLike::object(JsonObjectLike::new()))), |acc, (key, value)| {
            let key = match JsonLike::as_str(&key.0) {
                Some(key) => key,
                None => return ValR::Err(jaq_core::Error::str("Key cannot be converted to String")),
            };

            match acc {
                Ok(mut acc) => {
                    let acc_mut = JsonLike::as_object_mut(&mut acc.0).unwrap();
                    acc_mut.insert_key(key, value.0);
                    ValR::Ok(acc)
                },
                Err(err) => ValR::Err(err),
            }
        })
    }

    fn values(self) -> Box<dyn Iterator<Item = ValR<Self>>> {
        todo!()
    }

    fn index(self, index: &Self) -> ValR<Self> {
        if let Some(obj) = self.0.as_object() {
            let key = match index.0.as_str() {
                Some(key) => key,
                None => return ValR::Err(jaq_core::Error::str("Key cannot be converted to String"))
            };

            match obj.get_key(key) {
                Some(item) => ValR::Ok(JsonLikeHelper(item.clone())),
                None => ValR::Ok(JsonLikeHelper(JsonLike::null())),
            }
        } else if let Some(arr) = self.0.as_array() {
            let index: u64 = match index.0.as_u64() {
                Some(item) => item,
                None => return ValR::Err(jaq_core::Error::str("Index cannot be converted to u64"))
            };

            match arr.get(index as usize) {
                Some(item) => ValR::Ok(JsonLikeHelper(item.clone())),
                None => ValR::Ok(JsonLikeHelper(JsonLike::null())),
            }
        } else {
            ValR::Err(jaq_core::Error::str("Value is not object or array"))
        }
    }

    fn range(self, range: jaq_core::val::Range<&Self>) -> ValR<Self> {
        let (from, upto) = (range.start, range.end);
        if let Some(a) = self.0.clone().into_array() {
            let len = a.len();

            let from = from.as_ref().map(|i| i.as_i64()).flatten().ok_or_else(|| jaq_core::Error::str("From is not a Number"));
            let upto = upto.as_ref().map(|i| i.as_i64()).flatten().ok_or_else(|| jaq_core::Error::str("Upto is not a Number"));

            let (from, upto) = from.and_then(|from| Ok((from, upto?))).map(|(from, upto)| {
                let from: Result<isize, _> = from.try_into().map_err(|_| jaq_core::Error::str("From cannot be converted to isize"));
                let upto: Result<isize, _> = upto.try_into().map_err(|_| jaq_core::Error::str("Upto cannot be converted to isize"));
                (from, upto)
            })?;

            from.and_then(|from| Ok((from, upto?))).map(|(from, upto)| {
                let from = abs_bound(Some(from), len, 0);
                let upto = abs_bound(Some(upto), len, len);
                let (skip, take) = skip_take(from, upto);
                a.iter().skip(skip).take(take).cloned().map(JsonLikeHelper).collect()
            })
        } else if let Some(s) = self.0.clone().as_str() {
            let len = s.chars().count();

            let from = from.as_ref().map(|i| i.as_i64()).flatten().ok_or_else(|| jaq_core::Error::str("From is not a Number"));
            let upto = upto.as_ref().map(|i| i.as_i64()).flatten().ok_or_else(|| jaq_core::Error::str("Upto is not a Number"));

            let (from, upto) = from.and_then(|from| Ok((from, upto?))).map(|(from, upto)| {
                let from: Result<isize, _> = from.try_into().map_err(|_| jaq_core::Error::str("From cannot be converted to isize"));
                let upto: Result<isize, _> = upto.try_into().map_err(|_| jaq_core::Error::str("Upto cannot be converted to isize"));
                (from, upto)
            })?;

            from.and_then(|from| Ok((from, upto?))).map(|(from, upto)| {
                let from = abs_bound(Some(from), len, 0);
                let upto = abs_bound(Some(upto), len, len);
                let (skip, take) = skip_take(from, upto);
                JsonLikeHelper(JsonLike::string(s.chars().skip(skip).take(take).collect()))
            })
        } else {
            Err(jaq_core::Error::str("Value is not object or array"))
        }
    }

    fn map_values<'a, I: Iterator<Item = jaq_core::ValX<'a, Self>>>(
        self,
        opt: jaq_core::path::Opt,
        f: impl Fn(Self) -> I,
    ) -> jaq_core::ValX<'a, Self> {
        todo!()
    }

    fn map_index<'a, I: Iterator<Item = jaq_core::ValX<'a, Self>>>(
        self,
        index: &Self,
        opt: jaq_core::path::Opt,
        f: impl Fn(Self) -> I,
    ) -> jaq_core::ValX<'a, Self> {
        todo!()
    }

    fn map_range<'a, I: Iterator<Item = jaq_core::ValX<'a, Self>>>(
        self,
        range: jaq_core::val::Range<&Self>,
        opt: jaq_core::path::Opt,
        f: impl Fn(Self) -> I,
    ) -> jaq_core::ValX<'a, Self> {
        todo!()
    }

    fn as_bool(&self) -> bool {
        if let Some(b) = self.0.as_bool() {
            b
        } else if self.0.is_null() {
            false
        } else {
            true
        }
    }

    fn as_str(&self) -> Option<&str> {
        if let Some(s) = self.0.as_str() {
            Some(s)
        } else if let Some(b) = self.0.as_bool() {
            if b {
                Some("true")
            } else {
                Some("false")
            }
        } else {
            // TODO: fill the rest cases
            None
        }
    }
}

impl<A> std::fmt::Display for JsonLikeHelper<A> where A: for<'a> JsonLike<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        todo!()
    }
}

impl<A> From<bool> for JsonLikeHelper<A> where A: for<'a> JsonLike<'a> {
    fn from(value: bool) -> Self {
        todo!()
    }
}

impl<A> From<isize> for JsonLikeHelper<A> where A: for<'a> JsonLike<'a> {
    fn from(value: isize) -> Self {
        todo!()
    }
}

impl<A> From<String> for JsonLikeHelper<A> where A: for<'a> JsonLike<'a> {
    fn from(value: String) -> Self {
        todo!()
    }
}

impl<A> FromIterator<Self> for JsonLikeHelper<A> where A: for<'a> JsonLike<'a> {
    fn from_iter<T: IntoIterator<Item = Self>>(iter: T) -> Self {
        todo!()
    }
}

impl<A> std::ops::Add for JsonLikeHelper<A> where A: for<'a> JsonLike<'a> {
    type Output = ValR<Self>;
    fn add(self, rhs: Self) -> Self::Output {
        todo!()
    }
}

impl<A> std::ops::Sub for JsonLikeHelper<A> where A: for<'a> JsonLike<'a> {
    type Output = ValR<Self>;
    fn sub(self, rhs: Self) -> Self::Output {
        todo!()
    }
}

impl<A> std::ops::Mul for JsonLikeHelper<A> where A: for<'a> JsonLike<'a> {
    type Output = ValR<Self>;
    fn mul(self, rhs: Self) -> Self::Output {
        todo!()
    }
}

impl<A> std::ops::Div for JsonLikeHelper<A> where A: for<'a> JsonLike<'a> {
    type Output = ValR<Self>;
    fn div(self, rhs: Self) -> Self::Output {
        todo!()
    }
}

impl<A> std::ops::Rem for JsonLikeHelper<A> where A: for<'a> JsonLike<'a> {
    type Output = ValR<Self>;
    fn rem(self, rhs: Self) -> Self::Output {
        todo!()
    }
}

impl<A> std::ops::Neg for JsonLikeHelper<A> where A: for<'a> JsonLike<'a> {
    type Output = ValR<Self>;
    fn neg(self) -> Self::Output {
        todo!()
    }
}

fn skip_take(from: usize, until: usize) -> (usize, usize) {
    (from, if until > from { until - from } else { 0 })
}

/// If a range bound is given, absolutise and clip it between 0 and `len`,
/// else return `default`.
fn abs_bound(i: Option<isize>, len: usize, default: usize) -> usize {
    i.map_or(default, |i| core::cmp::min(wrap(i, len).unwrap_or(0), len))
}

/// Absolutise an index and return result if it is inside [0, len).
fn abs_index(i: isize, len: usize) -> Option<usize> {
    wrap(i, len).filter(|i| *i < len)
}

fn wrap(i: isize, len: usize) -> Option<usize> {
    if i >= 0 {
        Some(i as usize)
    } else if len < -i as usize {
        None
    } else {
        Some(len - (-i as usize))
    }
}
