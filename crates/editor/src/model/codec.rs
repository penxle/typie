use crate::model::id::NodeId;
use anyhow::Context;
use loro::{LoroList, LoroMap, LoroValue};

pub trait Codec: Sized {
    fn to_value(&self) -> Option<LoroValue> {
        None
    }

    fn from_value(_value: LoroValue) -> anyhow::Result<Self> {
        anyhow::bail!("Not a primitive type")
    }

    fn encode(&mut self, _map: &LoroMap) -> anyhow::Result<()> {
        anyhow::bail!("This type doesn't support encode")
    }

    fn decode(_map: &LoroMap) -> anyhow::Result<Self> {
        anyhow::bail!("This type doesn't support decode")
    }

    fn encode_field(&mut self, map: &LoroMap, key: &str) -> anyhow::Result<()> {
        if let Some(value) = self.to_value() {
            map.insert(key, value)?;
        } else {
            let field_map = map.insert_container(key, LoroMap::new())?;
            self.encode(&field_map)?;
        }
        Ok(())
    }

    fn decode_field(map: &LoroMap, key: &str) -> anyhow::Result<Self> {
        if let Some(value_or_container) = map.get(key) {
            if let Ok(value) = value_or_container.into_value() {
                if let Ok(result) = Self::from_value(value) {
                    return Ok(result);
                }
            }
        }

        let field_map = map
            .get(key)
            .with_context(|| format!("missing {}", key))?
            .into_container()
            .ok()
            .context("not a container")?
            .into_map()
            .ok()
            .context("not a map")?;
        Self::decode(&field_map)
    }

    fn encode_as_list_item(&mut self, list: &LoroList) -> anyhow::Result<()> {
        if let Some(value) = self.to_value() {
            list.push(value)?;
        } else {
            let item_map = list.push_container(LoroMap::new())?;
            self.encode(&item_map)?;
        }
        Ok(())
    }

    fn decode_from_list_item(value_or_container: loro::ValueOrContainer) -> anyhow::Result<Self> {
        if let Ok(value) = value_or_container.clone().into_value() {
            if let Ok(result) = Self::from_value(value) {
                return Ok(result);
            }
        }

        let item_map = value_or_container
            .into_container()
            .ok()
            .context("not a container")?
            .into_map()
            .ok()
            .context("not a map")?;
        Self::decode(&item_map)
    }
}

impl Codec for String {
    fn to_value(&self) -> Option<LoroValue> {
        Some(LoroValue::String(self.clone().into()))
    }

    fn from_value(value: LoroValue) -> anyhow::Result<Self> {
        match value {
            LoroValue::String(s) => Ok(s.to_string()),
            _ => anyhow::bail!("value not string"),
        }
    }
}

impl Codec for f32 {
    fn to_value(&self) -> Option<LoroValue> {
        Some(LoroValue::Double(*self as f64))
    }

    fn from_value(value: LoroValue) -> anyhow::Result<Self> {
        match value {
            LoroValue::Double(d) => Ok(d as f32),
            LoroValue::I64(i) => Ok(i as f32),
            _ => anyhow::bail!("value not number"),
        }
    }
}

impl Codec for u16 {
    fn to_value(&self) -> Option<LoroValue> {
        Some(LoroValue::I64(*self as i64))
    }

    fn from_value(value: LoroValue) -> anyhow::Result<Self> {
        match value {
            LoroValue::I64(i) => Ok(i as u16),
            LoroValue::Double(d) => Ok(d as u16),
            _ => anyhow::bail!("value not number"),
        }
    }
}

impl Codec for u64 {
    fn to_value(&self) -> Option<LoroValue> {
        Some(LoroValue::I64(*self as i64))
    }

    fn from_value(value: LoroValue) -> anyhow::Result<Self> {
        match value {
            LoroValue::I64(i) => Ok(i as u64),
            LoroValue::Double(d) => Ok(d as u64),
            _ => anyhow::bail!("value not number"),
        }
    }
}

impl Codec for bool {
    fn to_value(&self) -> Option<LoroValue> {
        Some(LoroValue::Bool(*self))
    }

    fn from_value(value: LoroValue) -> anyhow::Result<Self> {
        match value {
            LoroValue::Bool(b) => Ok(b),
            _ => anyhow::bail!("value not bool"),
        }
    }
}

impl<T: Codec> Codec for Vec<T> {
    fn encode_field(&mut self, map: &LoroMap, key: &str) -> anyhow::Result<()> {
        let list = map
            .insert_container(key, LoroList::new())
            .context("failed to create list")?;
        for item in self.iter_mut() {
            item.encode_as_list_item(&list)?;
        }
        Ok(())
    }

    fn decode_field(map: &LoroMap, key: &str) -> anyhow::Result<Self> {
        let list = map
            .get(key)
            .with_context(|| format!("missing {}", key))?
            .into_container()
            .ok()
            .context("not a container")?
            .into_list()
            .ok()
            .context("value not list")?;

        let mut result = Vec::new();
        for i in 0..list.len() {
            let item_value = list.get(i).context("failed to get list item")?;
            result.push(T::decode_from_list_item(item_value)?);
        }
        Ok(result)
    }
}

impl<T: Codec> Codec for Option<T> {
    fn encode_field(&mut self, map: &LoroMap, key: &str) -> anyhow::Result<()> {
        match self {
            Some(value) => {
                value.encode_field(map, key)?;
            }
            None => {}
        }
        Ok(())
    }

    fn decode_field(map: &LoroMap, key: &str) -> anyhow::Result<Self> {
        match map.get(key) {
            Some(_) => Ok(Some(T::decode_field(map, key)?)),
            None => Ok(None),
        }
    }
}

impl Codec for NodeId {
    fn to_value(&self) -> Option<LoroValue> {
        Some(LoroValue::String(self.to_string().into()))
    }

    fn from_value(value: LoroValue) -> anyhow::Result<Self> {
        let value_str = match value {
            LoroValue::String(s) => s.to_string(),
            _ => anyhow::bail!("value not string"),
        };

        NodeId::from_string(&value_str).context("failed to parse NodeId")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loro::LoroDoc;

    #[test]
    fn test_string_codec() {
        let doc = LoroDoc::new();
        let map = doc.get_map("test");

        let mut original = "hello".to_string();
        original.encode_field(&map, "value").unwrap();
        let decoded = String::decode_field(&map, "value").unwrap();

        assert_eq!(original, decoded);
    }

    #[test]
    fn test_f32_codec() {
        let doc = LoroDoc::new();
        let map = doc.get_map("test");

        let mut original = 3.14f32;
        original.encode_field(&map, "value").unwrap();
        let decoded = f32::decode_field(&map, "value").unwrap();

        assert_eq!(original, decoded);
    }
}
