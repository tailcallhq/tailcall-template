use std::{borrow::Cow, ops::Deref};

use jaq_core::ValR;

use crate::jsonlike::{JsonLike, JsonObjectLike};

#[derive(Debug, Clone, PartialEq)]
pub struct JsonLikeHelper<
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
>(pub A);

impl<A> Deref for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    type Target = A;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<A> From<A> for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    fn from(value: A) -> Self {
        Self(value)
    }
}

impl<A> jaq_core::ValT for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    fn from_num(n: &str) -> ValR<Self> {
        match n.parse::<f64>() {
            Ok(num) => ValR::Ok(JsonLikeHelper(A::number_f64(num))),
            Err(err) => ValR::Err(jaq_core::Error::str(format!(
                "Invalid number format: {err}"
            ))),
        }
    }

    fn from_map<I: IntoIterator<Item = (Self, Self)>>(iter: I) -> ValR<Self> {
        iter.into_iter().try_fold(
            Self(JsonLike::object(JsonObjectLike::new())),
            |mut acc, (key, value)| {
                let key = JsonLike::as_str(&key.0)
                    .ok_or_else(|| jaq_core::Error::str("Key cannot be converted to String"))?;
                JsonLike::as_object_mut(&mut acc.0)
                    .unwrap()
                    .insert_key(key, value.0);
                Ok(acc)
            },
        )
    }

    fn values(self) -> Box<dyn Iterator<Item = ValR<Self>>> {
        if let Some(arr) = self.0.as_array() {
            let owned_array: Vec<_> = arr.to_vec();
            Box::new(owned_array.into_iter().map(|a| Ok(JsonLikeHelper(a))))
        } else if let Some(obj) = self.0.as_object() {
            let owned_array: Vec<_> = obj.iter().map(|(_k, v)| v.clone()).collect();
            Box::new(owned_array.into_iter().map(|a| Ok(JsonLikeHelper(a))))
        } else {
            Box::new(core::iter::once(ValR::Err(jaq_core::Error::str(
                "Value is not object or array",
            ))))
        }
    }

    fn index(self, index: &Self) -> ValR<Self> {
        if let Some(obj) = self.0.as_object() {
            let Some(key) = index.0.as_str() else {
                return ValR::Err(jaq_core::Error::str("Key cannot be converted to String"));
            };

            match obj.get_key(key) {
                Some(item) => ValR::Ok(JsonLikeHelper(item.clone())),
                None => ValR::Ok(JsonLikeHelper(JsonLike::null())),
            }
        } else if let Some(arr) = self.0.as_array() {
            let Some(index) = index.0.as_u64() else {
                return ValR::Err(jaq_core::Error::str("Index cannot be converted to u64"));
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

            let from = from
                .as_ref()
                .and_then(|i| i.as_i64())
                .ok_or_else(|| jaq_core::Error::str("From is not a Number"));
            let upto = upto
                .as_ref()
                .and_then(|i| i.as_i64())
                .ok_or_else(|| jaq_core::Error::str("Upto is not a Number"));

            let (from, upto) = from
                .and_then(|from| Ok((from, upto?)))
                .map(|(from, upto)| {
                    let from: Result<isize, _> = from
                        .try_into()
                        .map_err(|_| jaq_core::Error::str("From cannot be converted to isize"));
                    let upto: Result<isize, _> = upto
                        .try_into()
                        .map_err(|_| jaq_core::Error::str("Upto cannot be converted to isize"));
                    (from, upto)
                })?;

            from.and_then(|from| Ok((from, upto?))).map(|(from, upto)| {
                let from = abs_bound(Some(from), len, 0);
                let upto = abs_bound(Some(upto), len, len);
                let (skip, take) = skip_take(from, upto);
                a.iter()
                    .skip(skip)
                    .take(take)
                    .cloned()
                    .map(JsonLikeHelper)
                    .collect()
            })
        } else if let Some(s) = self.0.clone().as_str() {
            let len = s.chars().count();

            let from = from
                .as_ref()
                .and_then(|i| i.as_i64())
                .ok_or_else(|| jaq_core::Error::str("From is not a Number"));
            let upto = upto
                .as_ref()
                .and_then(|i| i.as_i64())
                .ok_or_else(|| jaq_core::Error::str("Upto is not a Number"));

            let (from, upto) = from
                .and_then(|from| Ok((from, upto?)))
                .map(|(from, upto)| {
                    let from: Result<isize, _> = from
                        .try_into()
                        .map_err(|_| jaq_core::Error::str("From cannot be converted to isize"));
                    let upto: Result<isize, _> = upto
                        .try_into()
                        .map_err(|_| jaq_core::Error::str("Upto cannot be converted to isize"));
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
        if let Some(arr) = self.0.as_array() {
            let iter = arr.iter().map(|a| JsonLikeHelper(a.clone())).flat_map(f);
            Ok(iter.collect::<Result<_, _>>()?)
        } else if let Some(obj) = self.0.as_object() {
            let iter = obj
                .iter()
                .filter_map(|(k, v)| f(JsonLikeHelper(v.clone())).next().map(|v| Ok((k, v?.0))));
            let obj = A::obj(iter.collect::<Result<Vec<_>, jaq_core::Exn<_>>>()?);
            Ok(JsonLikeHelper(obj))
        } else {
            return opt.fail(self, |_v| {
                jaq_core::Exn::from(jaq_core::Error::str("Value is not object or array"))
            });
        }
    }

    fn map_index<'a, I: Iterator<Item = jaq_core::ValX<'a, Self>>>(
        mut self,
        index: &Self,
        opt: jaq_core::path::Opt,
        f: impl Fn(Self) -> I,
    ) -> jaq_core::ValX<'a, Self> {
        if let Some(obj) = self.0.as_object_mut() {
            let Some(key) = index.0.as_str() else {
                return opt.fail(self, |_v| {
                    jaq_core::Exn::from(jaq_core::Error::str("Key cannot be converted to String"))
                });
            };

            match obj.get_key(key) {
                Some(e) => match f(JsonLikeHelper(e.clone())).next().transpose()? {
                    Some(value) => obj.insert_key(key, value.0),
                    None => {
                        obj.remove_key(key);
                    }
                },
                None => {
                    if let Some(value) = f(JsonLikeHelper(JsonLike::null())).next().transpose()? {
                        obj.insert_key(key, value.0);
                    }
                }
            }
            Ok(self)
        } else if let Some(arr) = self.0.as_array_mut() {
            let Some(index) = index.0.as_u64() else {
                return opt.fail(self, |_v| {
                    jaq_core::Exn::from(jaq_core::Error::str("Index cannot be converted to u64"))
                });
            };
            let abs_or = |i| {
                abs_index(i, arr.len())
                    .ok_or(jaq_core::Error::str(format!("index {i} out of bounds")))
            };
            // TODO: perform error handling
            let index = match abs_or(index.try_into().unwrap()) {
                Ok(index) => index,
                Err(e) => return opt.fail(self, |_v| jaq_core::Exn::from(e)),
            };

            let item = arr[index].clone();
            if let Some(value) = f(JsonLikeHelper(item)).next().transpose()? {
                arr[index] = value.0;
            } else {
                arr.remove(index);
            }
            Ok(self)
        } else {
            return opt.fail(self, |_v| {
                jaq_core::Exn::from(jaq_core::Error::str("Value is not object or array"))
            });
        }
    }

    fn map_range<'a, I: Iterator<Item = jaq_core::ValX<'a, Self>>>(
        mut self,
        range: jaq_core::val::Range<&Self>,
        opt: jaq_core::path::Opt,
        f: impl Fn(Self) -> I,
    ) -> jaq_core::ValX<'a, Self> {
        if let Some(arr) = self.0.as_array_mut() {
            let len = arr.len();
            let from: Result<Option<isize>, jaq_core::Error<JsonLikeHelper<A>>> = range
                .start
                .as_ref()
                .as_ref()
                .and_then(|i| i.as_i64())
                .map(|v| {
                    v.try_into()
                        .map_err(|_| jaq_core::Error::str("From cannot be converted to isize"))
                })
                .transpose();
            let upto: Result<Option<isize>, jaq_core::Error<JsonLikeHelper<A>>> = range
                .end
                .as_ref()
                .as_ref()
                .and_then(|i| i.as_i64())
                .map(|v| {
                    v.try_into()
                        .map_err(|_| jaq_core::Error::str("From cannot be converted to isize"))
                })
                .transpose();

            let (Ok(from), Ok(upto)) = (from, upto) else {
                return opt.fail(self, |_v| {
                    jaq_core::Exn::from(jaq_core::Error::str("Failed to parse range"))
                });
            };

            let from = abs_bound(from, len, 0);
            let upto = abs_bound(upto, len, len);

            let (skip, take) = skip_take(from, upto);

            let arr_slice = arr
                .iter_mut()
                .skip(skip)
                .take(take)
                .map(|a| a.clone())
                .collect::<Vec<_>>();

            let new_values =
                f(JsonLikeHelper(JsonLike::array(arr_slice))).collect::<Result<Vec<_>, _>>()?;

            arr.splice(skip..skip + take, new_values.into_iter().map(|a| a.0));
            Ok(self)
        } else {
            opt.fail(self, |_v| {
                jaq_core::Exn::from(jaq_core::Error::str("Value is not array"))
            })
        }
    }

    fn as_bool(&self) -> bool {
        if let Some(b) = self.0.as_bool() {
            b
        } else {
            !self.0.is_null()
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
            // TODO: fill the rest cases if possible
            None
        }
    }
}

impl<A> PartialOrd for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // TODO: compare properly
        self.0.to_string().partial_cmp(&other.0.to_string())
    }
}

impl<A> std::fmt::Display for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl<A> From<bool> for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    fn from(_value: bool) -> Self {
        todo!()
    }
}

impl<A> From<isize> for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    fn from(_value: isize) -> Self {
        todo!()
    }
}

impl<A> From<String> for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    fn from(value: String) -> Self {
        JsonLikeHelper(JsonLike::string(Cow::Owned(value)))
    }
}

impl<A> FromIterator<Self> for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    fn from_iter<T: IntoIterator<Item = Self>>(_iter: T) -> Self {
        todo!()
    }
}

impl<A> std::ops::Add for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    type Output = ValR<Self>;
    fn add(mut self, rhs: Self) -> Self::Output {
        if self.0.is_null() && rhs.0.is_null() {
            return Ok(self);
        }

        if let (Some(l), Some(r)) = (self.0.as_f64(), &rhs.0.as_f64()) {
            return Ok(JsonLikeHelper(A::number_f64(l + r)));
        }

        if let (Some(l), Some(r)) = (self.0.as_str(), &rhs.0.as_str()) {
            let mut result = String::from(l);
            result.push_str(r);
            return Ok(JsonLikeHelper(A::string(result.into())));
        }

        if let (Some(l), Some(r)) = (self.0.as_array_mut(), &rhs.0.as_array()) {
            l.extend(r.iter().cloned());
            return Ok(self);
        }

        if let (Some(l), Some(r)) = (self.0.as_object_mut(), &rhs.0.as_object()) {
            for (k, v) in r.iter() {
                l.insert_key(k, v.clone());
            }
            return Ok(self);
        }

        Err(jaq_core::Error::str("Cannot add values of different types"))
    }
}

impl<A> std::ops::Sub for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    type Output = ValR<Self>;
    fn sub(self, _rhs: Self) -> Self::Output {
        todo!()
    }
}

impl<A> std::ops::Mul for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    type Output = ValR<Self>;
    fn mul(self, _rhs: Self) -> Self::Output {
        todo!()
    }
}

impl<A> std::ops::Div for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    type Output = ValR<Self>;
    fn div(self, _rhs: Self) -> Self::Output {
        todo!()
    }
}

impl<A> std::ops::Rem for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    type Output = ValR<Self>;
    fn rem(self, _rhs: Self) -> Self::Output {
        todo!()
    }
}

impl<A> std::ops::Neg for JsonLikeHelper<A>
where
    A: for<'a> JsonLike<'a> + std::fmt::Display + std::clone::Clone + std::cmp::PartialEq + 'static,
{
    type Output = ValR<Self>;
    fn neg(self) -> Self::Output {
        todo!()
    }
}

fn skip_take(from: usize, until: usize) -> (usize, usize) {
    (from, until.saturating_sub(from))
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
