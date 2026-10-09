//! Barcode generators: Code 128 (auto subsets B and C) and EAN-13 / EAN-8.
//! Output is a list of bar modules (1 = bar, 0 = space) that the caller
//! turns into rectangles.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Symbology {
    Code128,
    Ean13,
    Ean8,
}

impl Symbology {
    pub const ALL: [Symbology; 3] = [Symbology::Code128, Symbology::Ean13, Symbology::Ean8];

    pub fn name(self) -> &'static str {
        match self {
            Symbology::Code128 => "Code 128",
            Symbology::Ean13 => "EAN-13",
            Symbology::Ean8 => "EAN-8",
        }
    }
}

/// Encode `text`; returns the module pattern or an error message key.
pub fn encode(sym: Symbology, text: &str) -> Result<Vec<u8>, &'static str> {
    match sym {
        Symbology::Code128 => code128(text),
        Symbology::Ean13 => ean13(text),
        Symbology::Ean8 => ean8(text),
    }
}

// ----- Code 128 ------------------------------------------------------------------

/// Bar widths of the 107 Code 128 symbols (each 6 widths, except stop with 7).
const CODE128_WIDTHS: [&str; 107] = [
    "212222", "222122", "222221", "121223", "121322", "131222", "122213", "122312", "132212",
    "221213", "221312", "231212", "112232", "122132", "122231", "113222", "123122", "123221",
    "223211", "221132", "221231", "213212", "223112", "312131", "311222", "321122", "321221",
    "312212", "322112", "322211", "212123", "212321", "232121", "111323", "131123", "131321",
    "112313", "132113", "132311", "211313", "231113", "231311", "112133", "112331", "132131",
    "113123", "113321", "133121", "313121", "211331", "231131", "213113", "213311", "213131",
    "311123", "311321", "331121", "312113", "312311", "332111", "314111", "221411", "431111",
    "111224", "111422", "121124", "121421", "141122", "141221", "112214", "112412", "122114",
    "122411", "142112", "142211", "241211", "221114", "413111", "241112", "134111", "111242",
    "121142", "121241", "114212", "124112", "124211", "411212", "421112", "421211", "212141",
    "214121", "412121", "111143", "111341", "131141", "114113", "114311", "411113", "411311",
    "113141", "114131", "311141", "411131", "211412", "211214", "211232", "2331112",
];

const START_B: usize = 104;
const START_C: usize = 105;
const CODE_B: usize = 100;
const CODE_C: usize = 99;
const STOP: usize = 106;

fn code128(text: &str) -> Result<Vec<u8>, &'static str> {
    if text.is_empty() || !text.bytes().all(|b| (32..127).contains(&b)) {
        return Err("status.barcode_invalid");
    }
    let bytes = text.as_bytes();
    // Choose subsets: runs of 4+ digits (or all digits with even length) go to C.
    let mut codes: Vec<usize> = Vec::new();
    let mut i = 0;
    let digits_from =
        |i: usize| -> usize { bytes[i..].iter().take_while(|b| b.is_ascii_digit()).count() };
    let mut in_c = false;
    let all_digits_even = bytes.iter().all(|b| b.is_ascii_digit()) && bytes.len().is_multiple_of(2);
    if all_digits_even || digits_from(0) >= 4 {
        codes.push(START_C);
        in_c = true;
    } else {
        codes.push(START_B);
    }
    while i < bytes.len() {
        let run = digits_from(i);
        if in_c {
            if run >= 2 {
                let v = (bytes[i] - b'0') as usize * 10 + (bytes[i + 1] - b'0') as usize;
                codes.push(v);
                i += 2;
                continue;
            }
            codes.push(CODE_B);
            in_c = false;
        } else if run >= 4 && run % 2 == 0 || run >= 6 {
            codes.push(CODE_C);
            in_c = true;
            continue;
        }
        codes.push((bytes[i] - 32) as usize);
        i += 1;
    }
    // Checksum: start value + sum(position * value) mod 103.
    let mut sum = codes[0];
    for (k, c) in codes.iter().enumerate().skip(1) {
        sum += k * c;
    }
    codes.push(sum % 103);
    codes.push(STOP);
    let mut out = Vec::new();
    for c in codes {
        let mut bar = true;
        for w in CODE128_WIDTHS[c].bytes() {
            let n = (w - b'0') as usize;
            out.extend(std::iter::repeat_n(u8::from(bar), n));
            bar = !bar;
        }
    }
    Ok(out)
}

// ----- EAN -----------------------------------------------------------------------

const EAN_L: [&str; 10] = [
    "0001101", "0011001", "0010011", "0111101", "0100011", "0110001", "0101111", "0111011",
    "0110111", "0001011",
];
const EAN_G: [&str; 10] = [
    "0100111", "0110011", "0011011", "0100001", "0011101", "0111001", "0000101", "0010001",
    "0001001", "0010111",
];
const EAN_R: [&str; 10] = [
    "1110010", "1100110", "1101100", "1000010", "1011100", "1001110", "1010000", "1000100",
    "1001000", "1110100",
];
/// Parity pattern for the left half of EAN-13, indexed by the first digit (L = 0, G = 1).
const EAN13_PARITY: [&str; 10] = [
    "000000", "001011", "001101", "001110", "010011", "011001", "011100", "010101", "010110",
    "011010",
];

fn digits(text: &str) -> Option<Vec<u8>> {
    text.chars()
        .map(|c| c.to_digit(10).map(|d| d as u8))
        .collect()
}

/// EAN check digit for the leading digits (12 for EAN-13, 7 for EAN-8).
pub fn ean_check(d: &[u8]) -> u8 {
    let mut sum = 0u32;
    for (i, v) in d.iter().rev().enumerate() {
        sum += *v as u32 * if i % 2 == 0 { 3 } else { 1 };
    }
    ((10 - sum % 10) % 10) as u8
}

fn push_pattern(out: &mut Vec<u8>, p: &str) {
    out.extend(p.bytes().map(|b| b - b'0'));
}

fn ean13(text: &str) -> Result<Vec<u8>, &'static str> {
    let mut d = digits(text).ok_or("status.barcode_invalid")?;
    match d.len() {
        12 => d.push(ean_check(&d)),
        13 if ean_check(&d[..12]) == d[12] => {}
        _ => return Err("status.barcode_invalid"),
    }
    let mut out = Vec::new();
    push_pattern(&mut out, "101");
    let parity = EAN13_PARITY[d[0] as usize].as_bytes();
    for (k, v) in d[1..7].iter().enumerate() {
        let table = if parity[k] == b'0' { EAN_L } else { EAN_G };
        push_pattern(&mut out, table[*v as usize]);
    }
    push_pattern(&mut out, "01010");
    for v in &d[7..13] {
        push_pattern(&mut out, EAN_R[*v as usize]);
    }
    push_pattern(&mut out, "101");
    Ok(out)
}

fn ean8(text: &str) -> Result<Vec<u8>, &'static str> {
    let mut d = digits(text).ok_or("status.barcode_invalid")?;
    match d.len() {
        7 => d.push(ean_check(&d)),
        8 if ean_check(&d[..7]) == d[7] => {}
        _ => return Err("status.barcode_invalid"),
    }
    let mut out = Vec::new();
    push_pattern(&mut out, "101");
    for v in &d[..4] {
        push_pattern(&mut out, EAN_L[*v as usize]);
    }
    push_pattern(&mut out, "01010");
    for v in &d[4..8] {
        push_pattern(&mut out, EAN_R[*v as usize]);
    }
    push_pattern(&mut out, "101");
    Ok(out)
}

/// Modules -> bar rectangles as (x_offset_in_modules, width_in_modules).
pub fn bars(modules: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < modules.len() {
        if modules[i] == 1 {
            let start = i;
            while i < modules.len() && modules[i] == 1 {
                i += 1;
            }
            out.push((start, i - start));
        } else {
            i += 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ean13_known_vector() {
        // 590123412345 -> check digit 7.
        assert_eq!(ean_check(&[5, 9, 0, 1, 2, 3, 4, 1, 2, 3, 4, 5]), 7);
        let m = ean13("5901234123457").unwrap();
        assert_eq!(m.len(), 95);
        assert!(ean13("5901234123450").is_err());
    }

    #[test]
    fn ean8_length() {
        let m = ean8("9638507").unwrap();
        assert_eq!(m.len(), 67);
    }

    #[test]
    fn code128_checksum_and_shape() {
        // "ABC" in subset B: start 104, 33, 34, 35; check = (104 + 33 + 68 + 105) % 103 = 4.
        let m = code128("ABC").unwrap();
        // 4 data/start symbols + check + stop = 6 * 11 + 2 = 68 modules.
        assert_eq!(m.len(), 68);
        assert_eq!(m[0], 1);
        assert_eq!(*m.last().unwrap(), 1);
        // Digits use subset C and come out shorter.
        let d = code128("12345678").unwrap();
        assert!(d.len() < code128("1234567A").unwrap().len());
        assert!(code128("").is_err());
    }
}
