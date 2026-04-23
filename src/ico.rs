/// Pure-Rust ICO → PNG converter.
///
/// Parses an ICO container, picks the largest image entry, and returns
/// valid PNG bytes.  Handles two ICO entry flavours:
///   1. Embedded PNG – the entry payload is already a PNG; returned as-is.
///   2. Raw BMP/DIB – the entry stores an uncompressed BGRA bitmap with a
///      top-down DIB header; we flip rows, swizzle to RGBA, and encode a
///      minimal valid PNG (IHDR + IDAT + IEND) using store-only deflate
///      (no compression crate needed).
use std::error::Error;

// ── ICO directory structures ────────────────────────────────────────────

/// 6-byte ICO file header.
struct IcoHeader {
    /// Number of images in the file.
    count: u16,
}

/// 16-byte ICO directory entry.
struct IcoDirEntry {
    width: u32,
    height: u32,
    /// Byte offset of the image data from the start of the file.
    data_offset: u32,
    /// Size of the image data in bytes.
    data_size: u32,
}

// ── Little-endian readers ───────────────────────────────────────────────

fn read_u16_le(buf: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([buf[off], buf[off + 1]])
}

fn read_u32_le(buf: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([buf[off], buf[off + 1], buf[off + 2], buf[off + 3]])
}

// ── ICO parsing ─────────────────────────────────────────────────────────

fn parse_header(data: &[u8]) -> Result<IcoHeader, Box<dyn Error>> {
    if data.len() < 6 {
        return Err("ICO file too short for header".into());
    }
    let reserved = read_u16_le(data, 0);
    let img_type = read_u16_le(data, 2);
    if reserved != 0 || img_type != 1 {
        return Err("Not a valid ICO file".into());
    }
    Ok(IcoHeader {
        count: read_u16_le(data, 4),
    })
}

fn parse_entries(data: &[u8], count: u16) -> Result<Vec<IcoDirEntry>, Box<dyn Error>> {
    let mut entries = Vec::with_capacity(count as usize);
    for i in 0..count as usize {
        let base = 6 + i * 16;
        if base + 16 > data.len() {
            return Err("ICO file truncated in directory".into());
        }
        // Width/height: 0 in the directory byte means 256.
        let w = match data[base] {
            0 => 256u32,
            v => v as u32,
        };
        let h = match data[base + 1] {
            0 => 256u32,
            v => v as u32,
        };
        entries.push(IcoDirEntry {
            width: w,
            height: h,
            data_size: read_u32_le(data, base + 8),
            data_offset: read_u32_le(data, base + 12),
        });
    }
    Ok(entries)
}

/// Pick the entry with the largest pixel area.
fn largest_entry(entries: &[IcoDirEntry]) -> Option<&IcoDirEntry> {
    entries.iter().max_by_key(|e| e.width * e.height)
}

// ── PNG helpers ─────────────────────────────────────────────────────────

const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

/// CRC-32 (ISO 3309 / ITU-T V.42) used by PNG chunks.
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Adler-32 checksum used inside the zlib wrapper.
fn adler32(data: &[u8]) -> u32 {
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// Wrap raw bytes in a zlib stream using store-only (uncompressed) deflate
/// blocks.  Each block can hold at most 65 535 bytes.
fn zlib_store(data: &[u8]) -> Vec<u8> {
    // zlib header: CM=8 (deflate), CINFO=7 (32 K window), FCHECK so
    // that the 16-bit value is a multiple of 31.
    let cmf: u8 = 0x78; // deflate, 32 K window
    let flg: u8 = 0x01; // FCHECK=1 → (0x78 * 256 + 0x01) % 31 == 0

    let max_block: usize = 65535;
    let num_blocks = if data.is_empty() {
        1
    } else {
        (data.len() + max_block - 1) / max_block
    };
    // 2 (header) + sum of block overhead (5 each) + data + 4 (adler32)
    let mut out = Vec::with_capacity(2 + num_blocks * 5 + data.len() + 4);

    out.push(cmf);
    out.push(flg);

    let mut pos = 0;
    while pos < data.len() {
        let end = std::cmp::min(pos + max_block, data.len());
        let is_final = end == data.len();
        let len = (end - pos) as u16;
        let nlen = !len;

        out.push(if is_final { 0x01 } else { 0x00 });
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&nlen.to_le_bytes());
        out.extend_from_slice(&data[pos..end]);
        pos = end;
    }

    // Edge case: empty input still needs one (final, empty) block.
    if data.is_empty() {
        out.push(0x01); // BFINAL=1, BTYPE=00
        out.extend_from_slice(&0u16.to_le_bytes());
        out.extend_from_slice(&0xFFFFu16.to_le_bytes());
    }

    let checksum = adler32(data);
    out.extend_from_slice(&checksum.to_be_bytes());
    out
}

/// Build a single PNG chunk (length + type + data + CRC).
fn png_chunk(chunk_type: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let len = data.len() as u32;
    let mut buf = Vec::with_capacity(12 + data.len());
    buf.extend_from_slice(&len.to_be_bytes());
    buf.extend_from_slice(chunk_type);
    buf.extend_from_slice(data);
    // CRC covers type + data
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(chunk_type);
    crc_input.extend_from_slice(data);
    buf.extend_from_slice(&crc32(&crc_input).to_be_bytes());
    buf
}

/// Encode raw RGBA pixels (top-to-bottom, left-to-right) as a minimal PNG.
fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    // IHDR: width(4) + height(4) + bit_depth(1) + colour_type(1) +
    //       compression(1) + filter(1) + interlace(1) = 13 bytes.
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(6); // colour type: RGBA
    ihdr.push(0); // compression method
    ihdr.push(0); // filter method
    ihdr.push(0); // interlace: none

    // Build the filtered scanlines (filter byte 0 = None for every row).
    let stride = (width as usize) * 4;
    let mut raw_data = Vec::with_capacity((1 + stride) * height as usize);
    for y in 0..height as usize {
        raw_data.push(0); // filter type: None
        raw_data.extend_from_slice(&rgba[y * stride..(y + 1) * stride]);
    }

    let compressed = zlib_store(&raw_data);

    let mut png = Vec::new();
    png.extend_from_slice(&PNG_SIGNATURE);
    png.extend_from_slice(&png_chunk(b"IHDR", &ihdr));
    png.extend_from_slice(&png_chunk(b"IDAT", &compressed));
    png.extend_from_slice(&png_chunk(b"IEND", &[]));
    png
}

// ── BMP / DIB entry decoding ────────────────────────────────────────────

/// Decode a raw BMP/DIB ICO entry into RGBA pixels.
///
/// ICO BMP entries store a BITMAPINFOHEADER (40 bytes) followed by pixel
/// data.  The height field is doubled (image + AND mask).  Pixel rows are
/// stored bottom-to-top.  Colour channels are BGRA.
fn decode_bmp_entry(data: &[u8], width: u32, height: u32) -> Result<Vec<u8>, Box<dyn Error>> {
    if data.len() < 40 {
        return Err("BMP entry too short for DIB header".into());
    }

    let bpp = read_u16_le(data, 14); // bits per pixel
    let compression = read_u32_le(data, 16);

    if compression != 0 {
        return Err(format!("Unsupported BMP compression: {}", compression).into());
    }

    let pixel_offset: usize = 40; // right after the BITMAPINFOHEADER
    let stride = (width as usize) * 4;
    let pixel_count = (width * height) as usize * 4;

    match bpp {
        32 => {
            // BGRA, bottom-to-top rows
            if data.len() < pixel_offset + pixel_count {
                return Err("BMP entry pixel data truncated".into());
            }
            let mut rgba = vec![0u8; pixel_count];
            for y in 0..height as usize {
                let src_row = pixel_offset + (height as usize - 1 - y) * stride;
                let dst_row = y * stride;
                for x in 0..width as usize {
                    let si = src_row + x * 4;
                    let di = dst_row + x * 4;
                    rgba[di] = data[si + 2]; // R ← B
                    rgba[di + 1] = data[si + 1]; // G
                    rgba[di + 2] = data[si]; // B ← R
                    rgba[di + 3] = data[si + 3]; // A
                }
            }
            Ok(rgba)
        }
        24 => {
            // BGR, bottom-to-top, rows padded to 4-byte boundary
            let row_bytes = ((24 * width as usize + 31) / 32) * 4;
            let and_mask_offset = pixel_offset + row_bytes * height as usize;
            let and_row_bytes = ((width as usize + 31) / 32) * 4;

            if data.len() < pixel_offset + row_bytes * height as usize {
                return Err("BMP 24bpp pixel data truncated".into());
            }

            let has_and_mask = data.len() >= and_mask_offset + and_row_bytes * height as usize;

            let mut rgba = vec![0u8; pixel_count];
            for y in 0..height as usize {
                let src_row = pixel_offset + (height as usize - 1 - y) * row_bytes;
                let dst_row = y * stride;
                for x in 0..width as usize {
                    let si = src_row + x * 3;
                    let di = dst_row + x * 4;
                    rgba[di] = data[si + 2]; // R
                    rgba[di + 1] = data[si + 1]; // G
                    rgba[di + 2] = data[si]; // B

                    // Alpha from AND mask (1 = transparent, 0 = opaque)
                    if has_and_mask {
                        let and_row = and_mask_offset + (height as usize - 1 - y) * and_row_bytes;
                        let byte_idx = and_row + x / 8;
                        let bit_idx = 7 - (x % 8);
                        let transparent = (data[byte_idx] >> bit_idx) & 1;
                        rgba[di + 3] = if transparent == 1 { 0 } else { 255 };
                    } else {
                        rgba[di + 3] = 255;
                    }
                }
            }
            Ok(rgba)
        }
        _ => Err(format!("Unsupported BMP bit depth: {}", bpp).into()),
    }
}

// ── Public API ──────────────────────────────────────────────────────────

/// Returns `true` if `data` looks like an ICO file (reserved=0, type=1).
pub fn is_ico(data: &[u8]) -> bool {
    data.len() >= 6 && data[0] == 0 && data[1] == 0 && data[2] == 1 && data[3] == 0
}

/// Convert an ICO file to PNG bytes.
///
/// Extracts the largest image from the ICO container.  If the entry is
/// already an embedded PNG it is returned verbatim.  Otherwise the raw
/// BMP/DIB payload is decoded and re-encoded as a minimal RGBA PNG.
pub fn ico_to_png(data: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let header = parse_header(data)?;
    if header.count == 0 {
        return Err("ICO file contains no images".into());
    }

    let entries = parse_entries(data, header.count)?;
    let best = largest_entry(&entries).ok_or("No entries found")?;

    let start = best.data_offset as usize;
    let end = start + best.data_size as usize;
    if end > data.len() {
        return Err("ICO entry data extends past end of file".into());
    }

    let payload = &data[start..end];

    // Check if the payload is already a PNG.
    if payload.starts_with(&PNG_SIGNATURE) {
        return Ok(payload.to_vec());
    }

    // Otherwise treat it as a raw BMP/DIB entry.
    let rgba = decode_bmp_entry(payload, best.width, best.height)?;
    Ok(encode_png(best.width, best.height, &rgba))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test: a minimal 1×1 ICO with a 32-bpp BMP entry.
    #[test]
    fn test_minimal_ico_roundtrip() {
        // Build a tiny 1×1 ICO in memory.
        let mut ico = Vec::new();

        // ICO header: reserved=0, type=1, count=1
        ico.extend_from_slice(&[0, 0, 1, 0, 1, 0]);

        // Directory entry: 1×1, 0 color planes, 0 reserved, 32 bpp
        let bmp_header_size: u32 = 40;
        let pixel_data_size: u32 = 4; // 1 pixel × 4 bytes
        let data_size = bmp_header_size + pixel_data_size;
        let data_offset: u32 = 6 + 16; // after header + 1 entry

        ico.push(1); // width
        ico.push(1); // height
        ico.push(0); // colour count
        ico.push(0); // reserved
        ico.extend_from_slice(&1u16.to_le_bytes()); // colour planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        ico.extend_from_slice(&data_size.to_le_bytes());
        ico.extend_from_slice(&data_offset.to_le_bytes());

        // BMP DIB header (BITMAPINFOHEADER, 40 bytes)
        ico.extend_from_slice(&40u32.to_le_bytes()); // header size
        ico.extend_from_slice(&1u32.to_le_bytes()); // width
        ico.extend_from_slice(&2u32.to_le_bytes()); // height (doubled for ICO)
        ico.extend_from_slice(&1u16.to_le_bytes()); // planes
        ico.extend_from_slice(&32u16.to_le_bytes()); // bpp
        ico.extend_from_slice(&0u32.to_le_bytes()); // compression
        ico.extend_from_slice(&4u32.to_le_bytes()); // image size
        ico.extend_from_slice(&0u32.to_le_bytes()); // x ppm
        ico.extend_from_slice(&0u32.to_le_bytes()); // y ppm
        ico.extend_from_slice(&0u32.to_le_bytes()); // colours used
        ico.extend_from_slice(&0u32.to_le_bytes()); // important colours

        // 1 pixel BGRA: blue=0, green=128, red=255, alpha=255
        ico.extend_from_slice(&[0x00, 0x80, 0xFF, 0xFF]);

        assert!(is_ico(&ico));

        let png = ico_to_png(&ico).expect("conversion should succeed");

        // The result should be a valid PNG.
        assert!(png.starts_with(&PNG_SIGNATURE));

        // Very basic size sanity: IHDR(25) + IDAT(overhead+5bytes) + IEND(12) + sig(8)
        assert!(png.len() > 50);
    }
}
