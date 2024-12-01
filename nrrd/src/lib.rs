use bzip2::read::BzDecoder;
use core::fmt::Debug;
use flate2::read::GzDecoder;
use std::fs;
use std::io::{BufRead, Read};
use std::path::Path;
use std::str::FromStr;


type Metadata = std::collections::HashMap<String, String>;


pub trait FromBuffer: {
    fn from_buffer(buffer: &[u8], endian: NrrdEndian) -> Self;
}

macro_rules! impl_from_buffer {
    ($($t:ty),*) => {
        $(
            impl FromBuffer for $t {
                fn from_buffer(buffer: &[u8], endian: NrrdEndian) -> Self {
                    match endian {
                        NrrdEndian::BE => <$t>::from_be_bytes(buffer.try_into().unwrap()),
                        NrrdEndian::LE => <$t>::from_le_bytes(buffer.try_into().unwrap()),
                    }
                }
            }
        )*
    };
}

// Use the macro to implement FromBuffer for supported numeric types
impl_from_buffer!(i8, u8, i16, u16, i32, u32, i64, u64, f32, f64);


#[derive(Copy, Clone)]
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




impl NrrdType {
    fn from_string(string: &str) -> Result<NrrdType, String> {
        match string {
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
            _ => Err(format!("Unknown data type : '{}'", string)),
        }

    }

}

#[derive(Copy, Clone)]
pub enum NrrdEndian {
    BE,
    LE,
}

impl NrrdEndian {
    fn from_string(string: &str) -> Result<NrrdEndian, String> {
        match string.to_lowercase().as_str() {
            "little" => Ok(NrrdEndian::LE),
            "big" => Ok(NrrdEndian::BE),
            _ => Err(format!("Unkown endianess '{}'", string)),
        }
    }

}


#[derive(Copy, Clone)]
pub enum NrrdEncoding {
    RAW,
    ASCII,
    GZIP,
    BZIP2,
}

impl NrrdEncoding {
    fn from_string(string: &str) -> Result<NrrdEncoding, String> {
        match string.to_lowercase().as_str() {
            "raw" => Ok(NrrdEncoding::RAW),
            "txt" | "text" | "ascii" => Ok(NrrdEncoding::ASCII),
            "gz" | "gzip" => Ok(NrrdEncoding::GZIP),
            "bz2" | "bzip2" => Ok(NrrdEncoding::BZIP2),
            _ => Err(format!("Unsupported encoding {}", string)),
        }
    }
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



pub struct Nrrd {
    pub metadata: Metadata,
    pub data: NrrdData,
}


fn bytes_to_type<T: FromBuffer>(buffer: &[u8], endian: NrrdEndian) -> Vec<T> {

    let type_size = std::mem::size_of::<T>();

    assert!(
        buffer.len() % type_size == 0,
        "Buffer size is not a multiple of the size of type.",
    );

    buffer
       .chunks_exact(type_size)
       .map(|chunk| T::from_buffer(chunk, endian))
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



fn parse_array<T>(metadata: &Metadata, data: &[u8], endian: NrrdEndian) -> Vec<T>
where
    T: FromStr + FromBuffer + Clone,
    <T as FromStr>::Err: Debug,
{
    let encoding = NrrdEncoding::from_string(metadata["encoding"].as_str()).unwrap();

    let numbers: Vec<T> = match encoding {
        NrrdEncoding::ASCII => text_to_type::<T>(&data),
        NrrdEncoding::RAW => bytes_to_type::<T>(&data, endian),
        NrrdEncoding::BZIP2 => {
            let sizes = parse_list::<usize>(metadata["sizes"].as_str());

            let type_size = std::mem::size_of::<T>();
            let count: usize = sizes.iter().product();
            let mut output = Vec::with_capacity(count * type_size);
            let mut decompressor = BzDecoder::new(data);
            match decompressor.read_to_end(&mut output) {
                Ok(len) => {
                    assert!(len == count * type_size);
                    bytes_to_type::<T>(&output, endian)
                }
                Err(e) => panic!("{e}"),
            }
        }
        NrrdEncoding::GZIP => {
            let sizes = parse_list::<usize>(metadata["sizes"].as_str());
            let type_size = std::mem::size_of::<T>();
            let count: usize = sizes.iter().product();
            let mut output = Vec::with_capacity(count * type_size);
            let mut decompressor = GzDecoder::new(data);
            match decompressor.read_to_end(&mut output) {
                Ok(len) => {
                    assert!(len == count * type_size);
                    bytes_to_type::<T>(&output, endian)
                }
                Err(e) => panic!("{e}"),
            }
        }
    };

    //assert!(numbers.len() == count);
    numbers

}


pub fn read_data(metadata: &Metadata, buffer: &[u8]) -> NrrdData {

    let data_type = NrrdType::from_string(metadata["type"].as_str()).unwrap();
    let endian = NrrdEndian::from_string(metadata["endian"].as_str()).unwrap();
    // TODO: Maybe make decoder a generic too?

    match data_type {
        NrrdType::I8  => NrrdData::I8(parse_array::<i8>(&metadata, &buffer, endian)),
        NrrdType::U8  => NrrdData::U8(parse_array::<u8>(&metadata, &buffer, endian)),
        NrrdType::I16 => NrrdData::I16(parse_array::<i16>(&metadata, &buffer, endian)),
        NrrdType::U16 => NrrdData::U16(parse_array::<u16>(&metadata, &buffer, endian)),
        NrrdType::I32 => NrrdData::I32(parse_array::<i32>(&metadata, &buffer, endian)),
        NrrdType::U32 => NrrdData::U32(parse_array::<u32>(&metadata, &buffer, endian)),
        NrrdType::I64 => NrrdData::I64(parse_array::<i64>(&metadata, &buffer, endian)),
        NrrdType::U64 => NrrdData::U64(parse_array::<u64>(&metadata, &buffer, endian)),
        NrrdType::F32 => NrrdData::F32(parse_array::<f32>(&metadata, &buffer, endian)),
        NrrdType::F64 => NrrdData::F64(parse_array::<f64>(&metadata, &buffer, endian)),
        _ => panic!("Unkown data type encoding.")
    }
}

pub fn parse_list<T: std::str::FromStr>(data: &str) -> Vec<T>
where
    <T as FromStr>::Err: Debug,
{
    data.split(' ').map(|v| v.parse::<T>().unwrap()).collect()
}


pub fn read_metadata(buffer: &[u8]) -> (Metadata, usize) {

    let mut offset: usize = 0;
    let mut lines = buffer.lines();
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

    (metadata, offset)
}


pub fn read_file(path: &Path) -> (Metadata, NrrdData) {
    let buffer = fs::read(path).unwrap();
    let (metadata, offset) = read_metadata(&buffer);
    let data = read_data(&metadata, &buffer[offset..]);
    (metadata, data)
}


impl Nrrd {
    pub fn sizes(self: &Self) -> Vec<usize> {
        parse_list(&self.metadata["sizes"])
    }

    pub fn from_buffer(buf: &[u8]) -> Self {
        let (metadata, offset) = read_metadata(&buf);
        let data = read_data(&metadata, &buf[offset..]);
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

    fn vec_compare<T: std::cmp::PartialEq>(va: &[T], vb: &[T]) -> bool {
        (va.len() == vb.len()) &&  // zip stops at the shortest
         va.iter()
           .zip(vb)
           .all(|(a,b)| { *a == *b} )
    }

    macro_rules! test_read_data_le {
        ($data_type:ty, $variant:path, $type_aliases:expr, $numbers:expr) => {
            {
                let bytes: Vec<u8> = $numbers.iter().flat_map(|&num| num.to_le_bytes()).collect();

                for type_synonym in $type_aliases {
                    let metadata = Metadata::from([
                        ("type".to_string(), type_synonym.to_string()),
                        ("dimension".to_string(), "1".to_string()),
                        ("sizes".to_string(), $numbers.len().to_string()),
                        ("encoding".to_string(), "raw".to_string()),
                        ("endian".to_string(), "little".to_string()),
                    ]);

                    let data = read_data(&metadata, &bytes);

                    if let $variant(values) = data {
                        assert_eq!(vec_compare::<$data_type>(&$numbers, &values), true);
                    } else {
                        panic!("Wrong Variant.");
                    }
                }
            }
        };
    }



    #[test]
    fn parse_header() {
        let expected = Metadata::from([
            ("type".to_string(), "double".to_string()),
            ("dimension".to_string(), "1".to_string()),
            ("sizes".to_string(), "5".to_string()),
            ("encoding".to_string(), "ASCII".to_string()),
            ("endian".to_string(), "little".to_string()),
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

    #[test]
    fn test_read_data_le() {

        let numbers: [i8; 5] = [5, 10, 15, 20, 25];
        let aliases = ["signed char", "int8", "int8_t"];
        test_read_data_le!(i8, NrrdData::I8, aliases, numbers);

        let numbers: [u8; 5] = [5, 10, 15, 20, 25];
        let aliases = ["unsigned char", "uchar", "uint8", "uint8_t"];
        test_read_data_le!(u8, NrrdData::U8, aliases, numbers);

        let numbers: [i16; 5] = [5, 10, 15, 20, 25];
        let aliases = ["short", "short int", "signed short", "signed short int", "int16", "int16_t"];
        test_read_data_le!(i16, NrrdData::I16, aliases, numbers);

        let numbers: [u16; 5] = [5, 10, 15, 20, 25];
        let aliases = ["ushort", "unsigned short", "unsigned short int", "uint16", "uint16_t"];
        test_read_data_le!(u16, NrrdData::U16, aliases, numbers);

        let numbers: [i32; 5] = [5, 10, 15, 20, 25];
        let aliases = ["int", "signed int", "int32", "int32_t"];
        test_read_data_le!(i32, NrrdData::I32, aliases, numbers);

        let numbers: [u32; 5] = [5, 10, 15, 20, 25];
        let aliases = ["uint", "unsigned int", "uint32", "uint32_t"];
        test_read_data_le!(u32, NrrdData::U32, aliases, numbers);

        let numbers: [i64; 5] = [5, 10, 15, 20, 25];
        let aliases = ["longlong", "long long", "signed long long", "signed long long int", "int64", "int64_t"];
        test_read_data_le!(i64, NrrdData::I64, aliases, numbers);

        let numbers: [u64; 5] = [5, 10, 15, 20, 25];
        let aliases = ["ulonglong", "unsigned long long", "unsigned long long int", "uint64", "uint64_t"];
        test_read_data_le!(u64, NrrdData::U64, aliases, numbers);

        let numbers: [f32; 5] = [5.0, 10.0, 15.0, 20.0, 25.0];
        let aliases = ["float"];
        test_read_data_le!(f32, NrrdData::F32, aliases, numbers);

        let numbers: [f64; 5] = [5.0, 10.0, 15.0, 20.0, 25.0];
        let aliases = ["double"];
        test_read_data_le!(f64, NrrdData::F64, aliases, numbers);

    }
}
