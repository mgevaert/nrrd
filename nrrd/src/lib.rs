use bzip2::read::BzDecoder;
use core::fmt::Debug;
use flate2::read::GzDecoder;
use std::fs;
use std::io::{BufRead, Read};
use std::path::Path;
use std::str::FromStr;

type Metadata = std::collections::HashMap<String, String>;


pub trait FromBuffer: {
    fn from_le_buffer(buffer: &[u8]) -> Self;
}

impl FromBuffer for i8 {
    fn from_le_buffer(buffer: &[u8]) -> Self {
        i8::from_le_bytes(buffer.try_into().unwrap())
    }
}

impl FromBuffer for u8 {
    fn from_le_buffer(buffer: &[u8]) -> Self {
        u8::from_le_bytes(buffer.try_into().unwrap())
    }
}

impl FromBuffer for i16 {
    fn from_le_buffer(buffer: &[u8]) -> Self {
        i16::from_le_bytes(buffer.try_into().unwrap())
    }
}

impl FromBuffer for u16 {
    fn from_le_buffer(buffer: &[u8]) -> Self {
        u16::from_le_bytes(buffer.try_into().unwrap())
    }
}

impl FromBuffer for i32 {
    fn from_le_buffer(buffer: &[u8]) -> Self {
        i32::from_le_bytes(buffer.try_into().unwrap())
    }
}

impl FromBuffer for u32 {
    fn from_le_buffer(buffer: &[u8]) -> Self {
        u32::from_le_bytes(buffer.try_into().unwrap())
    }
}

impl FromBuffer for i64 {
    fn from_le_buffer(buffer: &[u8]) -> Self {
        i64::from_le_bytes(buffer.try_into().unwrap())
    }
}

impl FromBuffer for u64 {
    fn from_le_buffer(buffer: &[u8]) -> Self {
        u64::from_le_bytes(buffer.try_into().unwrap())
    }
}

impl FromBuffer for f32 {
    fn from_le_buffer(buffer: &[u8]) -> Self {
        f32::from_le_bytes(buffer.try_into().unwrap())
    }
}

impl FromBuffer for f64 {
    fn from_le_buffer(buffer: &[u8]) -> Self {
        f64::from_le_bytes(buffer.try_into().unwrap())
    }
}


/// NRDD data type variants as listed in https://teem.sourceforge.net/nrrd/format.html#type
pub enum NrrdType {
    I8,
    U8,
    I16,
    U16,
    I32,
    U32,
    I64,
    U64,
    F32,
    F64,
    BLOCK,
}

pub enum NrrdData {
    I8(Vec<i8>),
    U8(Vec<u8>),
    I16(Vec<i16>),
    U16(Vec<u16>),
    I32(Vec<i32>),
    U32(Vec<u32>),
    I64(Vec<i64>),
    U64(Vec<u64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
}

impl NrrdData {
    pub fn len(&self) -> usize {
        match self {
            NrrdData::I8(vec) => vec.len(),
            NrrdData::U8(vec) => vec.len(),
            NrrdData::I16(vec) => vec.len(),
            NrrdData::U16(vec) => vec.len(),
            NrrdData::I32(vec) => vec.len(),
            NrrdData::U32(vec) => vec.len(),
            NrrdData::I64(vec) => vec.len(),
            NrrdData::U64(vec) => vec.len(),
            NrrdData::F32(vec) => vec.len(),
            NrrdData::F64(vec) => vec.len(),
        }
    }
}

fn lex_data_type(str_type: &str) -> Result<NrrdType, String> {
    match str_type {
        "signed char"            |
        "int8"                   |
        "int8_t"                 => Ok(NrrdType::I8),
        "unsigned char"          |
        "uchar"                  |
        "uint8"                  |
        "uint8_t"                => Ok(NrrdType::U8),
        "short"                  |
        "short int"              |
        "signed short"           |
        "signed short int"       |
        "int16"                  |
        "int16_t"                => Ok(NrrdType::I16),
        "ushort"                 |
        "unsigned short"         |
        "unsigned short int"     |
        "uint16"                 |
        "uint16_t"               => Ok(NrrdType::U16),
        "int"                    |
        "signed int"             |
        "int32"                  |
        "int32_t"                => Ok(NrrdType::I32),
        "uint"                   |
        "unsigned int"           |
        "uint32"                 |
        "uint32_t"               => Ok(NrrdType::U32),
        "longlong"               |
        "long long"              |
        "long long int"          |
        "signed long long"       |
        "signed long long int"   |
        "int64"                  |
        "int64_t"                => Ok(NrrdType::I64),
        "ulonglong"              |
        "unsigned long long"     |
        "unsigned long long int" |
        "uint64"                 |
        "uint64_t"               => Ok(NrrdType::U64),
        "float"                  => Ok(NrrdType::F32),
        "double"                 => Ok(NrrdType::F64),
        "block"                  => Ok(NrrdType::BLOCK),
        _ => Err(format!("Unknown data type : '{}'", str_type)),
    }

}


pub struct Nrrd {
    pub metadata: Metadata,
    pub data: NrrdData,
}


fn bytes_to_type<T: FromBuffer>(buffer: &[u8]) -> Vec<T>{
   buffer
       .chunks(std::mem::size_of::<T>())
       .map(T::from_le_buffer)
       .collect()
}


fn text_to_type<T: FromStr>(buffer: &[u8]) -> Vec<T>
where <T as FromStr>::Err: Debug
{
    buffer
        .split(|x| *x as char == ' ' || *x as char == '\n')
        .filter(|x| !x.is_empty())
        .map(|v| unsafe { std::str::from_utf8_unchecked(v).parse::<T>().unwrap() })
        .collect()
}



fn parse_array<T>(metadata: &Metadata, data: &[u8]) -> Vec<T>
where
    T: FromStr + FromBuffer,
    <T as FromStr>::Err: Debug
{

    let sizes = parse_list::<usize>(metadata["sizes"].as_str());
    let count: usize = sizes.iter().product();

    let encoding = metadata["encoding"].as_str();
    let type_size = std::mem::size_of::<T>();

    let numbers: Vec<T> = match encoding {
        "ASCII" => text_to_type::<T>(&data),
        "raw" => bytes_to_type::<T>(&data),
        "bzip2" => {
            let mut output = Vec::with_capacity(count * type_size);
            let mut decompressor = BzDecoder::new(data);
            match decompressor.read_to_end(&mut output) {
                Ok(len) => {
                    assert!(len == count * type_size);
                    bytes_to_type::<T>(&output)
                }
                Err(e) => panic!("{e}"),
            }
        }
        "gzip" => {
            let mut output = Vec::with_capacity(count * type_size);
            let mut decompressor = GzDecoder::new(data);
            match decompressor.read_to_end(&mut output) {
                Ok(len) => {
                    assert!(len == count * type_size);
                    bytes_to_type::<T>(&output)
                }
                Err(e) => panic!("{e}"),
            }
        }
        _ => panic!("Unknown encoding: '{}'", encoding),
    };

    assert!(numbers.len() == count);
    numbers

}


fn parse_data(metadata: &Metadata, data: &[u8]) -> NrrdData {

    let data_type = lex_data_type(metadata["type"].as_str()).unwrap();

    match data_type {
        NrrdType::I8  => NrrdData::I8(parse_array::<i8>(&metadata, &data)),
        NrrdType::U8  => NrrdData::U8(parse_array::<u8>(&metadata, &data)),
        NrrdType::I16 => NrrdData::I16(parse_array::<i16>(&metadata, &data)),
        NrrdType::U16 => NrrdData::U16(parse_array::<u16>(&metadata, &data)),
        NrrdType::I32 => NrrdData::I32(parse_array::<i32>(&metadata, &data)),
        NrrdType::U32 => NrrdData::U32(parse_array::<u32>(&metadata, &data)),
        NrrdType::I64 => NrrdData::I64(parse_array::<i64>(&metadata, &data)),
        NrrdType::U64 => NrrdData::U64(parse_array::<u64>(&metadata, &data)),
        NrrdType::F32 => NrrdData::F32(parse_array::<f32>(&metadata, &data)),
        NrrdType::F64 => NrrdData::F64(parse_array::<f64>(&metadata, &data)),
        _ => panic!("Unkown data type encoding.")
    }
}

fn parse_list<T: std::str::FromStr>(data: &str) -> Vec<T>
where
    <T as FromStr>::Err: Debug,
{
    data.split(' ').map(|v| v.parse::<T>().unwrap()).collect()
}

impl Nrrd {
    pub fn sizes(self: &Self) -> Vec<usize> {
        parse_list(&self.metadata["sizes"])
    }

    pub fn from_buffer(buf: &[u8]) -> Self {
        let mut offset: usize = 0;
        let mut lines = buf.lines();
        if let Some(Ok(line)) = lines.next() {
            if line != "NRRD0003" && line != "NRRD0004" && line != "NRRD0005" {
                panic!("Incorrect magic line: '{}'", line);
            }
            offset += line.len() + 1;
        } else {
            panic!();
        }

        let mut metadata = Metadata::new();
        while let Some(Ok(line)) = lines.next() {
            offset += line.len() + 1;
            if line.is_empty() {
                break;
            }
            if let Some((k, v)) = line.split_once(':') {
                let k = k.trim().to_string();
                let mut v = v.trim().to_string();

                if k.starts_with('#') {
                    continue;
                } else if v.starts_with('=') {
                    v = v[1..].trim().to_string();
                }
                metadata.insert(k, v);
            }
        }
        if !metadata.contains_key("sizes") {
            panic!("Missing `sizes` in header")
        } else if !metadata.contains_key("encoding") {
            panic!("Missing `encoding` in header")
        } else if !metadata.contains_key("dimension") {
            panic!("Missing `dimension` in header")
        }

        let data = parse_data(
            &metadata,
            &buf[offset..],
        );

        Self { metadata, data }
    }

    pub fn from_file(path: &Path) -> Self {
        let buf = fs::read(path).unwrap();
        Self::from_buffer(&buf[..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_header() {
        let expected = Metadata::from([
            ("type".to_string(), "double".to_string()),
            ("dimension".to_string(), "1".to_string()),
            ("sizes".to_string(), "5".to_string()),
            ("encoding".to_string(), "ASCII".to_string()),
        ]);

        let nrrd = Nrrd::from_file(Path::new("../tests/data/test-headers.nrrd"));
        assert_eq!(nrrd.metadata, expected);

        if let NrrdData::F64(values) = nrrd.data {
            assert_eq!(values.iter().sum::<f64>(), 0. + 1. + 3. + 4. + 5.);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn parse_raw() {
        let expected = Metadata::from([
            ("type".to_string(), "double".to_string()),
            ("dimension".to_string(), "1".to_string()),
            ("sizes".to_string(), "5".to_string()),
            ("encoding".to_string(), "raw".to_string()),
            ("endian".to_string(), "little".to_string()),
        ]);

        let nrrd = Nrrd::from_file(Path::new("../tests/data/test-double-raw.nrrd"));
        assert_eq!(nrrd.metadata, expected);

        if let NrrdData::F64(values) = nrrd.data {
            assert_eq!(values.iter().sum::<f64>(), 0. + 1. + 2. + 3. + 4.);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn parse_binary_f64_bz2() {
        let expected = Metadata::from([
            ("type".to_string(), "double".to_string()),
            ("dimension".to_string(), "1".to_string()),
            ("sizes".to_string(), "5".to_string()),
            ("encoding".to_string(), "bzip2".to_string()),
            ("endian".to_string(), "little".to_string()),
        ]);
        let nrrd = Nrrd::from_file(Path::new("../tests/data/test-double-bz2.nrrd"));
        assert_eq!(nrrd.metadata, expected);
        if let NrrdData::F64(values) = nrrd.data {
            assert_eq!(values.iter().sum::<f64>(), 0. + 1. + 2. + 3. + 4.);
        } else {
            panic!("Wrong variant");
        }
    }

    #[test]
    fn parse_binary_f64_gz() {
        let expected = Metadata::from([
            ("type".to_string(), "double".to_string()),
            ("dimension".to_string(), "1".to_string()),
            ("sizes".to_string(), "5".to_string()),
            ("encoding".to_string(), "gzip".to_string()),
            ("endian".to_string(), "little".to_string()),
        ]);
        let nrrd = Nrrd::from_file(Path::new("../tests/data/test-double-gzip.nrrd"));
        assert_eq!(nrrd.metadata, expected);
        if let NrrdData::F64(values) = nrrd.data {
            assert_eq!(values.iter().sum::<f64>(), 0. + 1. + 2. + 3. + 4.);
        } else {
            panic!("Wrong variant");
        }
    }
}
