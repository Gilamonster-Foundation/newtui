//! Validate the JSON-expressible portable subset through Serde's parser.
//! This is not a codec: canonical encoding belongs to content-addressable.

use std::fmt;

use serde::de::{self, Deserialize, MapAccess, SeqAccess, Visitor};
use serde_json::Value;

pub(crate) fn validate(value: &Value) -> Result<(), String> {
    fn walk(value: &Value, depth: usize) -> Result<(), String> {
        if depth > 32 {
            return Err("portable values are limited to 32 nesting levels".into());
        }
        match value {
            Value::Number(number) if number.as_i64().is_none() => {
                Err("portable numbers must be signed 64-bit integers".into())
            }
            Value::Array(items) => items.iter().try_for_each(|item| walk(item, depth + 1)),
            Value::Object(fields) => fields.values().try_for_each(|item| walk(item, depth + 1)),
            _ => Ok(()),
        }
    }
    walk(value, 0)
}

struct Portable(Value);

impl<'de> Deserialize<'de> for Portable {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(PortableVisitor).map(Portable)
    }
}

struct PortableVisitor;

impl<'de> Visitor<'de> for PortableVisitor {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("null, bool, i64, string, array, or a map with unique string keys")
    }
    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_none<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        i64::try_from(value).map(Value::from).map_err(E::custom)
    }
    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_string<E: de::Error>(self, value: String) -> Result<Value, E> {
        Ok(value.into())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(Portable(item)) = sequence.next_element()? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut fields = serde_json::Map::new();
        while let Some((key, Portable(value))) = map.next_entry::<String, Portable>()? {
            if fields.insert(key.clone(), value).is_some() {
                return Err(de::Error::custom(format!(
                    "duplicate portable map key: {key:?}"
                )));
            }
        }
        Ok(Value::Object(fields))
    }
}

pub(crate) fn deserialize<'de, D: de::Deserializer<'de>>(
    deserializer: D,
) -> Result<Value, D::Error> {
    let Portable(value) = Portable::deserialize(deserializer)?;
    validate(&value).map_err(de::Error::custom)?;
    Ok(value)
}
