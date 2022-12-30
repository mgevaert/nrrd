use bzip2::read::BzDecoder;
use core::fmt::Debug;
use flate2::read::GzDecoder;
use std::fs;
use std::io::{BufRead, Read};
use std::path::Path;
use std::str;
use std::str::FromStr;

type Metadata = std::collections::HashMap<String, String>;

pub struct Nrrd {
    pub metadata: Metadata,
    pub data: Vec<f64>,
}

fn parse_data(sizes: Vec<usize>, encoding: &str, data: &[u8]) -> Vec<f64> {
    type T = f64;
    let type_size = std::mem::size_of::<T>();

    let count: usize = sizes.iter().product();

    let numbers: Vec<T> = match encoding {
        "ASCII" => data
            .split(|x| *x as char == ' ' || *x as char == '\n')
            .filter(|x| !x.is_empty())
            .map(|v| unsafe { str::from_utf8_unchecked(v).parse::<T>().unwrap() })
            .collect(),
        "raw" => data
            .chunks(type_size)
            .map(|b| T::from_le_bytes(b.try_into().unwrap()))
            .collect(),
        "bzip2" => {
            let mut output = Vec::with_capacity(count * type_size);
            let mut decompressor = BzDecoder::new(data);
            match decompressor.read_to_end(&mut output) {
                Ok(len) => {
                    assert!(len == count * type_size);
                    output
                        .chunks(type_size)
                        .map(|b| T::from_le_bytes(b.try_into().unwrap()))
                        .collect()
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
                    output
                        .chunks(type_size)
                        .map(|b| T::from_le_bytes(b.try_into().unwrap()))
                        .collect()
                }
                Err(e) => panic!("{e}"),
            }
        }
        _ => panic!("Unknown encoding: '{}'", encoding),
    };

    assert!(numbers.len() == count);
    numbers
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
            parse_list::<usize>(&metadata["sizes"]),
            &metadata["encoding"],
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
        assert_eq!(nrrd.data.iter().sum::<f64>(), 0. + 1. + 3. + 4. + 5.);
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
        assert_eq!(nrrd.data.iter().sum::<f64>(), 0. + 1. + 2. + 3. + 4.);
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
        assert_eq!(nrrd.data.iter().sum::<f64>(), 0. + 1. + 2. + 3. + 4.);
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
        assert_eq!(nrrd.data.iter().sum::<f64>(), 0. + 1. + 2. + 3. + 4.);
    }
}
