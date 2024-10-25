use pyo3::prelude::*;
use pyo3::types::{PyList, PyTuple};
use pyo3::exceptions::PyValueError;

struct DecodingResult {
    value: i64,
    offset: u32,
}

fn decode_unsigned_value_from_string(encoded: &[u8], offset: u32) -> DecodingResult {
    let mut value: i64 = 0;
    let mut consumed: u32 = 0;
    let mut byte: u8 = 0;
    while consumed == 0 || byte >= 0x20 {
        byte = encoded[(consumed + offset) as usize] - 63;
        value |= ((byte & 0x1f) as i64) << (consumed * 5);
        consumed += 1;
    }
    return DecodingResult{value: value, offset: offset + consumed};
}

fn decode_signed_value_from_string(encoded: &[u8], offset: u32) -> DecodingResult {
    let tmp_result: DecodingResult = decode_unsigned_value_from_string(encoded, offset);
    let tmp_value: i64 = tmp_result.value;
    if tmp_value & 1 == 1 {
        return DecodingResult{
            value: !(tmp_value >> 1),
            offset: tmp_result.offset
        }
    } else {
        return DecodingResult{
            value: tmp_value >> 1,
            offset: tmp_result.offset
        }
    }
}

fn encode_unsigned_number(num: i64) -> Vec<u8>  {
    let mut encoded: Vec<u8> = vec![];
    let mut tmp: i64 = num;
    while tmp >= 0x20 {
        encoded.push(((0x20 | (tmp as u8 & 0x1f)) + 63) as u8);
        tmp >>= 5;
    }
    encoded.push((tmp as u8 + 63) as u8);
    return encoded;
}

fn encode_signed_number(num: i64) -> Vec<u8> {
    let mut sgn_num: i64 = num << 1;
    if num < 0 {
        sgn_num = !sgn_num;
    }
    return encode_unsigned_number(sgn_num);
}

const YEAR2010: i64 = 1262304000;

#[pyfunction]
fn decode(input: String) -> PyResult<Vec<(i64, f64, f64)>> {
    let mut prev_vals: [i64; 3] = [YEAR2010, 0, 0];
    let mut vals: [i64; 3] = [0, 0, 0];
    let mut bytes_consumed: u32 = 0;
    let mut decoding_result: DecodingResult;
    let encoded: &[u8] = &input.as_bytes();
    let encoded_length: u32 = encoded.len() as u32;
    let mut output: Vec<(i64, f64, f64)> = Vec::new();
    
    while bytes_consumed < encoded_length {
        for i in 0..3 {
            if i == 0 && bytes_consumed != 0 {
                decoding_result = decode_unsigned_value_from_string(encoded, bytes_consumed);
            } else {
                decoding_result = decode_signed_value_from_string(encoded, bytes_consumed);
            }
            bytes_consumed = decoding_result.offset;
            let new_val = prev_vals[i] + decoding_result.value;
            vals[i] = new_val;
            prev_vals[i] = new_val;
        }
        output.push(
            (
                vals[0],
                (vals[1] as f64) / 1e5,
                (vals[2] as f64) / 1e5
            )
        );
    }
    Ok(output)
}

#[pyfunction]
fn encode(data: &PyList) -> PyResult<String> {
    let mut prev_timestamp: i64 = YEAR2010;
    let mut prev_latitude: f64 = 0.0;
    let mut prev_longitude: f64 = 0.0;

    let mut output: Vec<u8> = vec![];
    let mut is_first: bool = true;

    for point_object in data.iter() {
        let point_data = point_object.downcast::<PyTuple>()?;
        let timestamp: i64 = point_data[0].extract::<f64>()?.round() as i64;
        let timestamp_diff: i64 = timestamp - prev_timestamp;
        if is_first {
            output.append(&mut encode_signed_number(timestamp_diff));
            is_first = false;
        } else {
            if timestamp_diff < 0 {
                return Err(PyValueError::new_err("Input data is not sorted"));
            }
            output.append(&mut encode_unsigned_number(timestamp_diff));
        }
        
        let latitude: f64 = point_data[1].extract::<f64>()?;
        let latitude_diff: i64 = ((latitude - prev_latitude) * 1e5).round() as i64;
        output.append(&mut encode_signed_number(latitude_diff));

        let longitude: f64 = point_data[2].extract::<f64>()?;
        let longitude_diff: i64 = ((longitude - prev_longitude) * 1e5).round() as i64;
        output.append(&mut encode_signed_number(longitude_diff));

        prev_timestamp += timestamp_diff;
        prev_latitude += (latitude_diff as f64) / 1e5;
        prev_longitude += (longitude_diff as f64) / 1e5;
   
    }
    Ok(unsafe {String::from_utf8_unchecked(output)})
}

#[pymodule]
fn gps_data_codec(_py: Python<'_>, m: &PyModule) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(encode, m)?)?;
    m.add_function(wrap_pyfunction!(decode, m)?)?;
    Ok(())
}