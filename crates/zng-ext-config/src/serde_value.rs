//! This is a patched copy of <https://github.com/arcnmx/serde-value/tree/master>
//!
//! + Add support for `i128`.
//! + Fix Clippy warnings.
//!
//! Patches not contributed because the repository looks abandoned, there are good old pull requests without response.

use serde::Deserialize;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::hash::{Hash, Hasher};

pub use de::*;
pub use ser::*;

#[derive(Clone, Debug)]
pub enum Value {
    Bool(bool),

    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),

    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),

    F32(f32),
    F64(f64),

    Char(char),
    String(String),

    Unit,
    Option(Option<Box<Value>>),
    Newtype(Box<Value>),
    Seq(Vec<Value>),
    Map(BTreeMap<Value, Value>),
    Bytes(Vec<u8>),
}

impl Hash for Value {
    fn hash<H>(&self, hasher: &mut H)
    where
        H: Hasher,
    {
        self.discriminant().hash(hasher);
        match *self {
            Value::Bool(v) => v.hash(hasher),
            Value::U8(v) => v.hash(hasher),
            Value::U16(v) => v.hash(hasher),
            Value::U32(v) => v.hash(hasher),
            Value::U64(v) => v.hash(hasher),
            Value::U128(v) => v.hash(hasher),
            Value::I8(v) => v.hash(hasher),
            Value::I16(v) => v.hash(hasher),
            Value::I32(v) => v.hash(hasher),
            Value::I64(v) => v.hash(hasher),
            Value::I128(v) => v.hash(hasher),
            Value::F32(v) => about_eq_hash(v, EQ_GRANULARITY, hasher),
            Value::F64(v) => about_eq_hash(v as f32, EQ_GRANULARITY, hasher),
            Value::Char(v) => v.hash(hasher),
            Value::String(ref v) => v.hash(hasher),
            Value::Unit => {}
            Value::Option(ref v) => v.hash(hasher),
            Value::Newtype(ref v) => v.hash(hasher),
            Value::Seq(ref v) => v.hash(hasher),
            Value::Map(ref v) => v.hash(hasher),
            Value::Bytes(ref v) => v.hash(hasher),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, rhs: &Self) -> bool {
        match (self, rhs) {
            (&Value::Bool(v0), &Value::Bool(v1)) => v0 == v1,
            (&Value::U8(v0), &Value::U8(v1)) => v0 == v1,
            (&Value::U16(v0), &Value::U16(v1)) => v0 == v1,
            (&Value::U32(v0), &Value::U32(v1)) => v0 == v1,
            (&Value::U64(v0), &Value::U64(v1)) => v0 == v1,
            (&Value::U128(v0), &Value::U128(v1)) => v0 == v1,
            (&Value::I8(v0), &Value::I8(v1)) => v0 == v1,
            (&Value::I16(v0), &Value::I16(v1)) => v0 == v1,
            (&Value::I32(v0), &Value::I32(v1)) => v0 == v1,
            (&Value::I64(v0), &Value::I64(v1)) => v0 == v1,
            (&Value::I128(v0), &Value::I128(v1)) => v0 == v1,
            (&Value::F32(v0), &Value::F32(v1)) => about_eq(v0, v1, EQ_GRANULARITY),
            (&Value::F64(v0), &Value::F64(v1)) => about_eq(v0 as f32, v1 as f32, EQ_GRANULARITY),
            (&Value::Char(v0), &Value::Char(v1)) => v0 == v1,
            (Value::String(v0), Value::String(v1)) => v0 == v1,
            (&Value::Unit, &Value::Unit) => true,
            (Value::Option(v0), Value::Option(v1)) => v0 == v1,
            (Value::Newtype(v0), Value::Newtype(v1)) => v0 == v1,
            (Value::Seq(v0), Value::Seq(v1)) => v0 == v1,
            (Value::Map(v0), Value::Map(v1)) => v0 == v1,
            (Value::Bytes(v0), Value::Bytes(v1)) => v0 == v1,
            _ => false,
        }
    }
}

impl Ord for Value {
    fn cmp(&self, rhs: &Self) -> Ordering {
        match (self, rhs) {
            (&Value::Bool(v0), Value::Bool(v1)) => v0.cmp(v1),
            (&Value::U8(v0), Value::U8(v1)) => v0.cmp(v1),
            (&Value::U16(v0), Value::U16(v1)) => v0.cmp(v1),
            (&Value::U32(v0), Value::U32(v1)) => v0.cmp(v1),
            (&Value::U64(v0), Value::U64(v1)) => v0.cmp(v1),
            (&Value::U128(v0), Value::U128(v1)) => v0.cmp(v1),
            (&Value::I8(v0), Value::I8(v1)) => v0.cmp(v1),
            (&Value::I16(v0), Value::I16(v1)) => v0.cmp(v1),
            (&Value::I32(v0), Value::I32(v1)) => v0.cmp(v1),
            (&Value::I64(v0), Value::I64(v1)) => v0.cmp(v1),
            (&Value::I128(v0), Value::I128(v1)) => v0.cmp(v1),
            (&Value::F32(v0), &Value::F32(v1)) => about_eq_ord(v0, v1, EQ_GRANULARITY),
            (&Value::F64(v0), &Value::F64(v1)) => about_eq_ord(v0 as _, v1 as _, EQ_GRANULARITY),
            (&Value::Char(v0), Value::Char(v1)) => v0.cmp(v1),
            (Value::String(v0), Value::String(v1)) => v0.cmp(v1),
            (Value::Unit, Value::Unit) => Ordering::Equal,
            (Value::Option(v0), Value::Option(v1)) => v0.cmp(v1),
            (Value::Newtype(v0), Value::Newtype(v1)) => v0.cmp(v1),
            (Value::Seq(v0), Value::Seq(v1)) => v0.cmp(v1),
            (Value::Map(v0), Value::Map(v1)) => v0.cmp(v1),
            (Value::Bytes(v0), Value::Bytes(v1)) => v0.cmp(v1),
            (v0, v1) => v0.discriminant().cmp(&v1.discriminant()),
        }
    }
}

impl Value {
    fn discriminant(&self) -> usize {
        match *self {
            Value::Bool(..) => 0,
            Value::U8(..) => 1,
            Value::U16(..) => 2,
            Value::U32(..) => 3,
            Value::U64(..) => 4,
            Value::U128(..) => 5,
            Value::I8(..) => 6,
            Value::I16(..) => 7,
            Value::I32(..) => 8,
            Value::I64(..) => 9,
            Value::I128(..) => 10,
            Value::F32(..) => 11,
            Value::F64(..) => 12,
            Value::Char(..) => 13,
            Value::String(..) => 14,
            Value::Unit => 15,
            Value::Option(..) => 16,
            Value::Newtype(..) => 17,
            Value::Seq(..) => 18,
            Value::Map(..) => 19,
            Value::Bytes(..) => 20,
        }
    }

    fn unexpected(&self) -> serde::de::Unexpected<'_> {
        match *self {
            Value::Bool(b) => serde::de::Unexpected::Bool(b),
            Value::U8(n) => serde::de::Unexpected::Unsigned(n as u64),
            Value::U16(n) => serde::de::Unexpected::Unsigned(n as u64),
            Value::U32(n) => serde::de::Unexpected::Unsigned(n as u64),
            Value::U64(n) => serde::de::Unexpected::Unsigned(n),
            Value::U128(_) => serde::de::Unexpected::Other("u128"),
            Value::I8(n) => serde::de::Unexpected::Signed(n as i64),
            Value::I16(n) => serde::de::Unexpected::Signed(n as i64),
            Value::I32(n) => serde::de::Unexpected::Signed(n as i64),
            Value::I64(n) => serde::de::Unexpected::Signed(n),
            Value::I128(_) => serde::de::Unexpected::Other("i128"),
            Value::F32(n) => serde::de::Unexpected::Float(n as f64),
            Value::F64(n) => serde::de::Unexpected::Float(n),
            Value::Char(c) => serde::de::Unexpected::Char(c),
            Value::String(ref s) => serde::de::Unexpected::Str(s),
            Value::Unit => serde::de::Unexpected::Unit,
            Value::Option(_) => serde::de::Unexpected::Option,
            Value::Newtype(_) => serde::de::Unexpected::NewtypeStruct,
            Value::Seq(_) => serde::de::Unexpected::Seq,
            Value::Map(_) => serde::de::Unexpected::Map,
            Value::Bytes(ref b) => serde::de::Unexpected::Bytes(b),
        }
    }

    pub fn deserialize_into<'de, T: Deserialize<'de>>(self) -> Result<T, DeserializerError> {
        T::deserialize(self)
    }
}

impl Eq for Value {}
impl PartialOrd for Value {
    fn partial_cmp(&self, rhs: &Self) -> Option<Ordering> {
        Some(self.cmp(rhs))
    }
}

use zng_unit::{EQ_GRANULARITY, about_eq, about_eq_hash, about_eq_ord};

mod ser {
    use serde::ser;
    use std::collections::BTreeMap;
    use std::error::Error;
    use std::fmt;

    use super::Value;

    #[derive(Debug)]
    pub enum SerializerError {
        Custom(String),
    }

    impl fmt::Display for SerializerError {
        fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                SerializerError::Custom(ref s) => fmt.write_str(s),
            }
        }
    }

    impl Error for SerializerError {
        fn description(&self) -> &str {
            "Value serializer error"
        }
    }

    impl ser::Error for SerializerError {
        fn custom<T: fmt::Display>(msg: T) -> SerializerError {
            SerializerError::Custom(msg.to_string())
        }
    }

    impl ser::Serialize for Value {
        fn serialize<S: ser::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            match *self {
                Value::Bool(v) => s.serialize_bool(v),
                Value::U8(v) => s.serialize_u8(v),
                Value::U16(v) => s.serialize_u16(v),
                Value::U32(v) => s.serialize_u32(v),
                Value::U64(v) => s.serialize_u64(v),
                Value::U128(v) => s.serialize_u128(v),
                Value::I8(v) => s.serialize_i8(v),
                Value::I16(v) => s.serialize_i16(v),
                Value::I32(v) => s.serialize_i32(v),
                Value::I64(v) => s.serialize_i64(v),
                Value::I128(v) => s.serialize_i128(v),
                Value::F32(v) => s.serialize_f32(v),
                Value::F64(v) => s.serialize_f64(v),
                Value::Char(v) => s.serialize_char(v),
                Value::String(ref v) => s.serialize_str(v),
                Value::Unit => s.serialize_unit(),
                Value::Option(None) => s.serialize_none(),
                Value::Option(Some(ref v)) => s.serialize_some(v),
                Value::Newtype(ref v) => s.serialize_newtype_struct("", v),
                Value::Seq(ref v) => v.serialize(s),
                Value::Map(ref v) => v.serialize(s),
                Value::Bytes(ref v) => s.serialize_bytes(v),
            }
        }
    }

    pub fn to_value<T: ser::Serialize>(value: T) -> Result<Value, SerializerError> {
        value.serialize(Serializer)
    }

    struct Serializer;

    impl ser::Serializer for Serializer {
        type Ok = Value;
        type Error = SerializerError;
        type SerializeSeq = SerializeSeq;
        type SerializeTuple = SerializeTuple;
        type SerializeTupleStruct = SerializeTupleStruct;
        type SerializeTupleVariant = SerializeTupleVariant;
        type SerializeMap = SerializeMap;
        type SerializeStruct = SerializeStruct;
        type SerializeStructVariant = SerializeStructVariant;

        fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Bool(v))
        }

        fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
            Ok(Value::I8(v))
        }

        fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
            Ok(Value::I16(v))
        }

        fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
            Ok(Value::I32(v))
        }

        fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
            Ok(Value::I64(v))
        }

        fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
            Ok(Value::I128(v))
        }

        fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
            Ok(Value::U8(v))
        }

        fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
            Ok(Value::U16(v))
        }

        fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
            Ok(Value::U32(v))
        }

        fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
            Ok(Value::U64(v))
        }

        fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
            Ok(Value::U128(v))
        }

        fn serialize_f32(self, v: f32) -> Result<Self::Ok, Self::Error> {
            Ok(Value::F32(v))
        }

        fn serialize_f64(self, v: f64) -> Result<Self::Ok, Self::Error> {
            Ok(Value::F64(v))
        }

        fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Char(v))
        }

        fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
            Ok(Value::String(v.to_string()))
        }

        fn serialize_bytes(self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Bytes(v.to_vec()))
        }

        fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Option(None))
        }

        fn serialize_some<T>(self, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            value.serialize(Serializer).map(|v| Value::Option(Some(Box::new(v))))
        }

        fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Unit)
        }

        fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Unit)
        }

        fn serialize_unit_variant(self, _name: &'static str, _variant_index: u32, variant: &'static str) -> Result<Self::Ok, Self::Error> {
            Ok(Value::String(variant.to_string()))
        }

        fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<Self::Ok, Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            value.serialize(Serializer).map(|v| Value::Newtype(Box::new(v)))
        }

        fn serialize_newtype_variant<T>(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            value: &T,
        ) -> Result<Self::Ok, Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            value.serialize(Serializer).map(|v| {
                let mut map = BTreeMap::new();
                map.insert(Value::String(variant.to_string()), v);
                Value::Map(map)
            })
        }

        fn serialize_seq(self, _len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
            Ok(SerializeSeq(vec![]))
        }

        fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
            Ok(SerializeTuple(vec![]))
        }

        fn serialize_tuple_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeTupleStruct, Self::Error> {
            Ok(SerializeTupleStruct(vec![]))
        }

        fn serialize_tuple_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            len: usize,
        ) -> Result<Self::SerializeTupleVariant, Self::Error> {
            Ok(SerializeTupleVariant(Value::String(variant.to_string()), Vec::with_capacity(len)))
        }

        fn serialize_map(self, _len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
            Ok(SerializeMap {
                map: BTreeMap::new(),
                key: None,
            })
        }

        fn serialize_struct(self, _name: &'static str, _len: usize) -> Result<Self::SerializeStruct, Self::Error> {
            Ok(SerializeStruct(BTreeMap::new()))
        }

        fn serialize_struct_variant(
            self,
            _name: &'static str,
            _variant_index: u32,
            variant: &'static str,
            _len: usize,
        ) -> Result<Self::SerializeStructVariant, Self::Error> {
            Ok(SerializeStructVariant(Value::String(variant.to_string()), BTreeMap::new()))
        }
    }

    struct SerializeSeq(Vec<Value>);

    impl ser::SerializeSeq for SerializeSeq {
        type Ok = Value;
        type Error = SerializerError;

        fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            let value = value.serialize(Serializer)?;
            self.0.push(value);
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Seq(self.0))
        }
    }

    struct SerializeTuple(Vec<Value>);

    impl ser::SerializeTuple for SerializeTuple {
        type Ok = Value;
        type Error = SerializerError;

        fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            let value = value.serialize(Serializer)?;
            self.0.push(value);
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Seq(self.0))
        }
    }

    struct SerializeTupleStruct(Vec<Value>);

    impl ser::SerializeTupleStruct for SerializeTupleStruct {
        type Ok = Value;
        type Error = SerializerError;

        fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            let value = value.serialize(Serializer)?;
            self.0.push(value);
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Seq(self.0))
        }
    }

    struct SerializeTupleVariant(Value, Vec<Value>);

    impl ser::SerializeTupleVariant for SerializeTupleVariant {
        type Ok = Value;
        type Error = SerializerError;

        fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            let value = value.serialize(Serializer)?;
            self.1.push(value);
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            let mut map = BTreeMap::new();
            map.insert(self.0, Value::Seq(self.1));
            Ok(Value::Map(map))
        }
    }

    struct SerializeMap {
        map: BTreeMap<Value, Value>,
        key: Option<Value>,
    }

    impl ser::SerializeMap for SerializeMap {
        type Ok = Value;
        type Error = SerializerError;

        fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            let key = key.serialize(Serializer)?;
            self.key = Some(key);
            Ok(())
        }

        fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            let value = value.serialize(Serializer)?;
            self.map.insert(self.key.take().unwrap(), value);
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Map(self.map))
        }
    }

    struct SerializeStruct(BTreeMap<Value, Value>);

    impl ser::SerializeStruct for SerializeStruct {
        type Ok = Value;
        type Error = SerializerError;

        fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            let key = Value::String(key.to_string());
            let value = value.serialize(Serializer)?;
            self.0.insert(key, value);
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            Ok(Value::Map(self.0))
        }
    }

    struct SerializeStructVariant(Value, BTreeMap<Value, Value>);

    impl ser::SerializeStructVariant for SerializeStructVariant {
        type Ok = Value;
        type Error = SerializerError;

        fn serialize_field<T>(&mut self, key: &'static str, value: &T) -> Result<(), Self::Error>
        where
            T: ser::Serialize + ?Sized,
        {
            let key = Value::String(key.to_string());
            let value = value.serialize(Serializer)?;
            self.1.insert(key, value);
            Ok(())
        }

        fn end(self) -> Result<Self::Ok, Self::Error> {
            let mut map = BTreeMap::new();
            map.insert(self.0, Value::Map(self.1));
            Ok(Value::Map(map))
        }
    }
}

mod de {
    use serde::{de, forward_to_deserialize_any};
    use std::collections::BTreeMap;
    use std::error::Error;
    use std::fmt;
    use std::marker::PhantomData;

    use super::Value;

    #[derive(Debug)]
    pub enum Unexpected {
        Bool(bool),
        Unsigned(u64),
        Signed(i64),
        Float(f64),
        Char(char),
        Str(String),
        Bytes(Vec<u8>),
        Unit,
        Option,
        NewtypeStruct,
        Seq,
        Map,
        Enum,
        UnitVariant,
        NewtypeVariant,
        TupleVariant,
        StructVariant,
        Other(String),
    }

    impl<'a> From<de::Unexpected<'a>> for Unexpected {
        fn from(unexp: de::Unexpected) -> Unexpected {
            match unexp {
                de::Unexpected::Bool(v) => Unexpected::Bool(v),
                de::Unexpected::Unsigned(v) => Unexpected::Unsigned(v),
                de::Unexpected::Signed(v) => Unexpected::Signed(v),
                de::Unexpected::Float(v) => Unexpected::Float(v),
                de::Unexpected::Char(v) => Unexpected::Char(v),
                de::Unexpected::Str(v) => Unexpected::Str(v.to_owned()),
                de::Unexpected::Bytes(v) => Unexpected::Bytes(v.to_owned()),
                de::Unexpected::Unit => Unexpected::Unit,
                de::Unexpected::Option => Unexpected::Option,
                de::Unexpected::NewtypeStruct => Unexpected::NewtypeStruct,
                de::Unexpected::Seq => Unexpected::Seq,
                de::Unexpected::Map => Unexpected::Map,
                de::Unexpected::Enum => Unexpected::Enum,
                de::Unexpected::UnitVariant => Unexpected::UnitVariant,
                de::Unexpected::NewtypeVariant => Unexpected::NewtypeVariant,
                de::Unexpected::TupleVariant => Unexpected::TupleVariant,
                de::Unexpected::StructVariant => Unexpected::StructVariant,
                de::Unexpected::Other(v) => Unexpected::Other(v.to_owned()),
            }
        }
    }

    impl Unexpected {
        pub fn to_unexpected<'a>(&'a self) -> de::Unexpected<'a> {
            match *self {
                Unexpected::Bool(v) => de::Unexpected::Bool(v),
                Unexpected::Unsigned(v) => de::Unexpected::Unsigned(v),
                Unexpected::Signed(v) => de::Unexpected::Signed(v),
                Unexpected::Float(v) => de::Unexpected::Float(v),
                Unexpected::Char(v) => de::Unexpected::Char(v),
                Unexpected::Str(ref v) => de::Unexpected::Str(v),
                Unexpected::Bytes(ref v) => de::Unexpected::Bytes(v),
                Unexpected::Unit => de::Unexpected::Unit,
                Unexpected::Option => de::Unexpected::Option,
                Unexpected::NewtypeStruct => de::Unexpected::NewtypeStruct,
                Unexpected::Seq => de::Unexpected::Seq,
                Unexpected::Map => de::Unexpected::Map,
                Unexpected::Enum => de::Unexpected::Enum,
                Unexpected::UnitVariant => de::Unexpected::UnitVariant,
                Unexpected::NewtypeVariant => de::Unexpected::NewtypeVariant,
                Unexpected::TupleVariant => de::Unexpected::TupleVariant,
                Unexpected::StructVariant => de::Unexpected::StructVariant,
                Unexpected::Other(ref v) => de::Unexpected::Other(v),
            }
        }
    }

    #[derive(Debug)]
    pub enum DeserializerError {
        Custom(String),
        InvalidType(Unexpected, String),
        InvalidValue(Unexpected, String),
        InvalidLength(usize, String),
        UnknownVariant(String, &'static [&'static str]),
        UnknownField(String, &'static [&'static str]),
        MissingField(&'static str),
        DuplicateField(&'static str),
    }

    impl de::Error for DeserializerError {
        fn custom<T: fmt::Display>(msg: T) -> Self {
            DeserializerError::Custom(msg.to_string())
        }

        fn invalid_type(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
            DeserializerError::InvalidType(unexp.into(), exp.to_string())
        }

        fn invalid_value(unexp: de::Unexpected, exp: &dyn de::Expected) -> Self {
            DeserializerError::InvalidValue(unexp.into(), exp.to_string())
        }

        fn invalid_length(len: usize, exp: &dyn de::Expected) -> Self {
            DeserializerError::InvalidLength(len, exp.to_string())
        }

        fn unknown_variant(field: &str, expected: &'static [&'static str]) -> Self {
            DeserializerError::UnknownVariant(field.into(), expected)
        }

        fn unknown_field(field: &str, expected: &'static [&'static str]) -> Self {
            DeserializerError::UnknownField(field.into(), expected)
        }

        fn missing_field(field: &'static str) -> Self {
            DeserializerError::MissingField(field)
        }

        fn duplicate_field(field: &'static str) -> Self {
            DeserializerError::DuplicateField(field)
        }
    }

    impl DeserializerError {
        pub fn to_error<E: de::Error>(&self) -> E {
            match *self {
                DeserializerError::Custom(ref msg) => E::custom(msg.clone()),
                DeserializerError::InvalidType(ref unexp, ref exp) => E::invalid_type(unexp.to_unexpected(), &&**exp),
                DeserializerError::InvalidValue(ref unexp, ref exp) => E::invalid_value(unexp.to_unexpected(), &&**exp),
                DeserializerError::InvalidLength(len, ref exp) => E::invalid_length(len, &&**exp),
                DeserializerError::UnknownVariant(ref field, exp) => E::unknown_variant(field, exp),
                DeserializerError::UnknownField(ref field, exp) => E::unknown_field(field, exp),
                DeserializerError::MissingField(field) => E::missing_field(field),
                DeserializerError::DuplicateField(field) => E::missing_field(field),
            }
        }

        pub fn into_error<E: de::Error>(self) -> E {
            self.to_error()
        }
    }

    impl Error for DeserializerError {
        fn description(&self) -> &str {
            "Value deserializer error"
        }
    }

    impl fmt::Display for DeserializerError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match *self {
                DeserializerError::Custom(ref msg) => write!(f, "{msg}"),
                DeserializerError::InvalidType(ref u, ref exp) => {
                    write!(f, "Invalid type {}. Expected {}", u.to_unexpected(), exp)
                }
                DeserializerError::InvalidValue(ref u, ref exp) => {
                    write!(f, "Invalid value {}. Expected {}", u.to_unexpected(), exp)
                }
                DeserializerError::InvalidLength(len, ref exp) => {
                    write!(f, "Invalid length {len}. Expected {exp}")
                }
                DeserializerError::UnknownVariant(ref field, exp) => {
                    write!(f, "Unknown variant {}. Expected one of {}", field, exp.join(", "))
                }
                DeserializerError::UnknownField(ref field, exp) => {
                    write!(f, "Unknown field {}. Expected one of {}", field, exp.join(", "))
                }
                DeserializerError::MissingField(field) => write!(f, "Missing field {field}"),
                DeserializerError::DuplicateField(field) => write!(f, "Duplicate field {field}"),
            }
        }
    }

    impl From<de::value::Error> for DeserializerError {
        fn from(e: de::value::Error) -> DeserializerError {
            DeserializerError::Custom(e.to_string())
        }
    }

    pub struct ValueVisitor;

    impl<'de> de::Visitor<'de> for ValueVisitor {
        type Value = Value;

        fn expecting(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
            fmt.write_str("any value")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
            Ok(Value::Bool(value))
        }

        fn visit_i8<E>(self, value: i8) -> Result<Value, E> {
            Ok(Value::I8(value))
        }

        fn visit_i16<E>(self, value: i16) -> Result<Value, E> {
            Ok(Value::I16(value))
        }

        fn visit_i32<E>(self, value: i32) -> Result<Value, E> {
            Ok(Value::I32(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
            Ok(Value::I64(value))
        }

        fn visit_i128<E>(self, v: i128) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Value::I128(v))
        }

        fn visit_u8<E>(self, value: u8) -> Result<Value, E> {
            Ok(Value::U8(value))
        }

        fn visit_u16<E>(self, value: u16) -> Result<Value, E> {
            Ok(Value::U16(value))
        }

        fn visit_u32<E>(self, value: u32) -> Result<Value, E> {
            Ok(Value::U32(value))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
            Ok(Value::U64(value))
        }

        fn visit_f32<E>(self, value: f32) -> Result<Value, E> {
            Ok(Value::F32(value))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
            Ok(Value::F64(value))
        }

        fn visit_u128<E>(self, v: u128) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Value::U128(v))
        }

        fn visit_char<E>(self, value: char) -> Result<Value, E> {
            Ok(Value::Char(value))
        }

        fn visit_str<E>(self, value: &str) -> Result<Value, E> {
            Ok(Value::String(value.into()))
        }

        fn visit_string<E>(self, value: String) -> Result<Value, E> {
            Ok(Value::String(value))
        }

        fn visit_unit<E>(self) -> Result<Value, E> {
            Ok(Value::Unit)
        }

        fn visit_none<E>(self) -> Result<Value, E> {
            Ok(Value::Option(None))
        }

        fn visit_some<D: de::Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
            d.deserialize_any(ValueVisitor).map(|v| Value::Option(Some(Box::new(v))))
        }

        fn visit_newtype_struct<D: de::Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
            d.deserialize_any(ValueVisitor).map(|v| Value::Newtype(Box::new(v)))
        }

        fn visit_seq<V: de::SeqAccess<'de>>(self, mut visitor: V) -> Result<Value, V::Error> {
            let mut values = Vec::new();
            while let Some(elem) = visitor.next_element()? {
                values.push(elem);
            }
            Ok(Value::Seq(values))
        }

        fn visit_map<V: de::MapAccess<'de>>(self, mut visitor: V) -> Result<Value, V::Error> {
            let mut values = BTreeMap::new();
            while let Some((key, value)) = visitor.next_entry()? {
                values.insert(key, value);
            }
            Ok(Value::Map(values))
        }

        fn visit_bytes<E>(self, v: &[u8]) -> Result<Value, E> {
            Ok(Value::Bytes(v.into()))
        }

        fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Value, E> {
            Ok(Value::Bytes(v))
        }
    }

    impl<'de> de::Deserialize<'de> for Value {
        fn deserialize<D: de::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            d.deserialize_any(ValueVisitor)
        }
    }

    impl<'de> de::IntoDeserializer<'de, DeserializerError> for Value {
        type Deserializer = Value;

        fn into_deserializer(self) -> Value {
            self
        }
    }

    pub struct ValueDeserializer<E> {
        value: Value,
        error: PhantomData<fn() -> E>,
    }

    impl<E> ValueDeserializer<E> {
        pub fn new(value: Value) -> Self {
            ValueDeserializer {
                value,
                error: Default::default(),
            }
        }
    }

    impl<'de, E> de::Deserializer<'de> for ValueDeserializer<E>
    where
        E: de::Error,
    {
        type Error = E;

        fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            match self.value {
                Value::Bool(v) => visitor.visit_bool(v),
                Value::U8(v) => visitor.visit_u8(v),
                Value::U16(v) => visitor.visit_u16(v),
                Value::U32(v) => visitor.visit_u32(v),
                Value::U64(v) => visitor.visit_u64(v),
                Value::U128(v) => visitor.visit_u128(v),
                Value::I8(v) => visitor.visit_i8(v),
                Value::I16(v) => visitor.visit_i16(v),
                Value::I32(v) => visitor.visit_i32(v),
                Value::I64(v) => visitor.visit_i64(v),
                Value::I128(v) => visitor.visit_i128(v),
                Value::F32(v) => visitor.visit_f32(v),
                Value::F64(v) => visitor.visit_f64(v),
                Value::Char(v) => visitor.visit_char(v),
                Value::String(v) => visitor.visit_string(v),
                Value::Unit => visitor.visit_unit(),
                Value::Option(None) => visitor.visit_none(),
                Value::Option(Some(v)) => visitor.visit_some(ValueDeserializer::new(*v)),
                Value::Newtype(v) => visitor.visit_newtype_struct(ValueDeserializer::new(*v)),
                Value::Seq(v) => visitor.visit_seq(de::value::SeqDeserializer::new(v.into_iter().map(ValueDeserializer::new))),
                Value::Map(v) => visitor.visit_map(de::value::MapDeserializer::new(
                    v.into_iter().map(|(k, v)| (ValueDeserializer::new(k), ValueDeserializer::new(v))),
                )),
                Value::Bytes(v) => visitor.visit_byte_buf(v),
            }
        }

        fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            match self.value {
                Value::Option(..) => self.deserialize_any(visitor),
                Value::Unit => visitor.visit_unit(),
                _ => visitor.visit_some(self),
            }
        }

        fn deserialize_enum<V: de::Visitor<'de>>(
            self,
            _name: &'static str,
            _variants: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            let (variant, value) = match self.value {
                Value::Map(value) => {
                    let mut iter = value.into_iter();
                    let (variant, value) = match iter.next() {
                        Some(v) => v,
                        None => {
                            return Err(de::Error::invalid_value(de::Unexpected::Map, &"map with a single key"));
                        }
                    };
                    // enums are encoded as maps with a single key:value pair
                    if iter.next().is_some() {
                        return Err(de::Error::invalid_value(de::Unexpected::Map, &"map with a single key"));
                    }
                    (variant, Some(value))
                }
                Value::String(variant) => (Value::String(variant), None),
                other => {
                    return Err(de::Error::invalid_type(other.unexpected(), &"string or map"));
                }
            };

            let d = EnumDeserializer {
                variant,
                value,
                error: Default::default(),
            };
            visitor.visit_enum(d)
        }

        fn deserialize_newtype_struct<V: de::Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, Self::Error> {
            match self.value {
                Value::Newtype(v) => visitor.visit_newtype_struct(ValueDeserializer::new(*v)),
                _ => visitor.visit_newtype_struct(self),
            }
        }

        forward_to_deserialize_any! {
            bool u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64 char str string unit
            seq bytes byte_buf map unit_struct
            tuple_struct struct tuple ignored_any identifier
        }
    }

    impl<'de, E> de::IntoDeserializer<'de, E> for ValueDeserializer<E>
    where
        E: de::Error,
    {
        type Deserializer = Self;

        fn into_deserializer(self) -> Self::Deserializer {
            self
        }
    }

    impl<'de> de::Deserializer<'de> for Value {
        type Error = DeserializerError;

        fn deserialize_any<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            ValueDeserializer::new(self).deserialize_any(visitor)
        }

        fn deserialize_option<V: de::Visitor<'de>>(self, visitor: V) -> Result<V::Value, Self::Error> {
            ValueDeserializer::new(self).deserialize_option(visitor)
        }

        fn deserialize_enum<V: de::Visitor<'de>>(
            self,
            name: &'static str,
            variants: &'static [&'static str],
            visitor: V,
        ) -> Result<V::Value, Self::Error> {
            ValueDeserializer::new(self).deserialize_enum(name, variants, visitor)
        }

        fn deserialize_newtype_struct<V: de::Visitor<'de>>(self, name: &'static str, visitor: V) -> Result<V::Value, Self::Error> {
            ValueDeserializer::new(self).deserialize_newtype_struct(name, visitor)
        }

        forward_to_deserialize_any! {
            bool u8 u16 u32 u64 u128 i8 i16 i32 i64 i128 f32 f64 char str string unit
            seq bytes byte_buf map unit_struct
            tuple_struct struct tuple ignored_any identifier
        }
    }

    struct EnumDeserializer<E> {
        variant: Value,
        value: Option<Value>,
        error: PhantomData<fn() -> E>,
    }

    impl<'de, E> de::EnumAccess<'de> for EnumDeserializer<E>
    where
        E: de::Error,
    {
        type Error = E;
        type Variant = VariantDeserializer<Self::Error>;

        fn variant_seed<V>(self, seed: V) -> Result<(V::Value, VariantDeserializer<Self::Error>), Self::Error>
        where
            V: de::DeserializeSeed<'de>,
        {
            let visitor = VariantDeserializer {
                value: self.value,
                error: Default::default(),
            };
            seed.deserialize(ValueDeserializer::new(self.variant)).map(|v| (v, visitor))
        }
    }

    struct VariantDeserializer<E> {
        value: Option<Value>,
        error: PhantomData<fn() -> E>,
    }

    impl<'de, E> de::VariantAccess<'de> for VariantDeserializer<E>
    where
        E: de::Error,
    {
        type Error = E;

        fn unit_variant(self) -> Result<(), Self::Error> {
            match self.value {
                Some(value) => de::Deserialize::deserialize(ValueDeserializer::new(value)),
                None => Ok(()),
            }
        }

        fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
        where
            T: de::DeserializeSeed<'de>,
        {
            match self.value {
                Some(value) => seed.deserialize(ValueDeserializer::new(value)),
                None => Err(de::Error::invalid_type(de::Unexpected::UnitVariant, &"newtype variant")),
            }
        }

        fn tuple_variant<V>(self, _len: usize, visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            match self.value {
                Some(Value::Seq(v)) => {
                    de::Deserializer::deserialize_any(de::value::SeqDeserializer::new(v.into_iter().map(ValueDeserializer::new)), visitor)
                }
                Some(other) => Err(de::Error::invalid_type(other.unexpected(), &"tuple variant")),
                None => Err(de::Error::invalid_type(de::Unexpected::UnitVariant, &"tuple variant")),
            }
        }

        fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value, Self::Error>
        where
            V: de::Visitor<'de>,
        {
            match self.value {
                Some(Value::Map(v)) => de::Deserializer::deserialize_any(
                    de::value::MapDeserializer::new(v.into_iter().map(|(k, v)| (ValueDeserializer::new(k), ValueDeserializer::new(v)))),
                    visitor,
                ),
                Some(other) => Err(de::Error::invalid_type(other.unexpected(), &"struct variant")),
                None => Err(de::Error::invalid_type(de::Unexpected::UnitVariant, &"struct variant")),
            }
        }
    }
}
