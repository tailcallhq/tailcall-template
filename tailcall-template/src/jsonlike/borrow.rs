use std::borrow::Cow;

use serde_json_borrow::{ObjectAsVec, Value};

use super::{gather_path_matches, group_by_key, JsonLike, JsonObjectLike};

// BorrowedValue
impl<'ctx> JsonObjectLike<'ctx> for ObjectAsVec<'ctx> {
    type Value = Value<'ctx>;

    fn new() -> Self {
        ObjectAsVec::default()
    }

    fn with_capacity(n: usize) -> Self {
        // TODO: no way to define capacity on ObjectAsVec
        ObjectAsVec::from(Vec::with_capacity(n))
    }

    fn get_key(&self, key: &str) -> Option<&Self::Value> {
        self.get(key)
    }

    fn insert_key(&mut self, key: &'ctx str, value: Self::Value) {
        self.insert(key, value);
    }

    fn remove_key(&mut self, _key: &'ctx str) -> Option<Self::Value> {
        // TODO: implement it
        unimplemented!()
    }

    fn iter(&'ctx self) -> impl Iterator<Item = (&'ctx str, &'ctx Self::Value)> {
        self.iter()
    }
}

impl<'ctx> JsonLike<'ctx> for Value<'ctx> {
    type JsonObject = ObjectAsVec<'ctx>;

    fn null() -> Self {
        Value::Null
    }

    fn object(obj: Self::JsonObject) -> Self {
        Value::Object(obj)
    }

    fn obj(pairs: Vec<(&'ctx str, Self)>) -> Self {
        Value::Object(ObjectAsVec::from(pairs))
    }

    fn array(arr: Vec<Self>) -> Self {
        Value::Array(arr)
    }

    fn string(s: Cow<'ctx, str>) -> Self {
        Value::Str(s)
    }

    fn number_f64(n: f64) -> Self {
        Value::Number(n.into())
    }

    fn as_array(&self) -> Option<&Vec<Self>> {
        match self {
            Value::Array(array) => Some(array),
            _ => None,
        }
    }

    fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            Value::Array(arr) => Some(arr),
            _ => None,
        }
    }

    fn into_array(self) -> Option<Vec<Self>> {
        match self {
            Value::Array(array) => Some(array),
            _ => None,
        }
    }

    fn as_object(&self) -> Option<&Self::JsonObject> {
        self.as_object()
    }

    fn as_object_mut(&mut self) -> Option<&mut Self::JsonObject> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }

    fn into_object(self) -> Option<Self::JsonObject> {
        match self {
            Value::Object(obj) => Some(obj),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        self.as_str()
    }

    fn as_i64(&self) -> Option<i64> {
        self.as_i64()
    }

    fn as_u64(&self) -> Option<u64> {
        self.as_u64()
    }

    fn as_f64(&self) -> Option<f64> {
        self.as_f64()
    }

    fn as_bool(&self) -> Option<bool> {
        self.as_bool()
    }

    fn is_null(&self) -> bool {
        self.is_null()
    }

    fn get_path<T: AsRef<str>>(&'ctx self, path: &[T]) -> Option<&'ctx Self> {
        let mut val = self;
        for token in path {
            val = match val {
                Value::Array(arr) => {
                    let index = token.as_ref().parse::<usize>().ok()?;
                    arr.get(index)?
                }
                Value::Object(map) => map.get(token.as_ref())?,
                _ => return None,
            };
        }
        Some(val)
    }

    fn get_key(&'ctx self, path: &str) -> Option<&'ctx Self> {
        match self {
            Value::Object(map) => map.get(path),
            _ => None,
        }
    }

    fn group_by(&'ctx self, path: &[String]) -> std::collections::HashMap<String, Vec<&'ctx Self>> {
        let src = gather_path_matches(self, path, vec![]);
        group_by_key(src)
    }
}
