//! Image cache and decode pipeline for NexBrowser.
//!
//! Decodes image bytes (PNG, JPEG, WebP, GIF) to RGBA pixel buffers,
//! caches decoded images by URL, and provides Vello-compatible output.
//!
//! ## T1 Primitive Grounding
//!
//! | Concept | Primitive | Symbol |
//! |---------|-----------|--------|
//! | Decode | Mapping | μ |
//! | Dimensions | Quantity | N |
//! | Cache key | Comparison | κ |
//! | Pixel data | Persistence | π |
//! | Placement | Location | λ |
//!
//! ## Tier Classification
//!
//! - `DecodedImage`: T2-P (μ + N + π)
//! - `ImageCache`: T2-C (κ + π + μ)

use std::collections::BTreeMap;

/// A decoded image ready for GPU rendering.
///
/// Tier: T2-P (bytes μ→ pixels, N×N dimensions)
#[derive(Debug, Clone)]
pub struct DecodedImage {
    /// RGBA pixel data (4 bytes per pixel, row-major).
    pub rgba_data: Vec<u8>,
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
    pub height: u32,
}

impl DecodedImage {
    /// Decode image bytes (PNG, JPEG, WebP, GIF) to RGBA.
    ///
    /// Returns `None` if the format is unsupported or data is corrupt.
    #[must_use]
    pub fn decode(bytes: &[u8]) -> Option<Self> {
        let img = image::load_from_memory(bytes).ok()?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        Some(Self {
            rgba_data: rgba.into_raw(),
            width,
            height,
        })
    }

    /// Total pixel count.
    #[must_use]
    pub fn pixel_count(&self) -> u32 {
        self.width * self.height
    }

    /// Total byte size of RGBA data.
    #[must_use]
    pub fn byte_size(&self) -> usize {
        self.rgba_data.len()
    }

    /// Create a 1x1 placeholder image (magenta = missing texture).
    #[must_use]
    pub fn placeholder() -> Self {
        Self {
            rgba_data: vec![255, 0, 255, 255], // Magenta
            width: 1,
            height: 1,
        }
    }

    /// Scale image to fit within max dimensions while preserving aspect ratio.
    ///
    /// Returns a new `DecodedImage` if scaling is needed, or `None` if
    /// the image already fits.
    #[must_use]
    pub fn scale_to_fit(&self, max_width: u32, max_height: u32) -> Option<Self> {
        if self.width <= max_width && self.height <= max_height {
            return None; // Already fits
        }

        let scale_x = max_width as f64 / self.width as f64;
        let scale_y = max_height as f64 / self.height as f64;
        let scale = scale_x.min(scale_y);

        let new_w = (self.width as f64 * scale).round() as u32;
        let new_h = (self.height as f64 * scale).round() as u32;

        if new_w == 0 || new_h == 0 {
            return None;
        }

        // Nearest-neighbor resize (fast, good enough for thumbnails)
        let mut data = vec![0u8; (new_w * new_h * 4) as usize];
        for y in 0..new_h {
            for x in 0..new_w {
                let src_x = ((x as f64 / scale).floor() as u32).min(self.width - 1);
                let src_y = ((y as f64 / scale).floor() as u32).min(self.height - 1);
                let src_idx = ((src_y * self.width + src_x) * 4) as usize;
                let dst_idx = ((y * new_w + x) * 4) as usize;
                data[dst_idx..dst_idx + 4].copy_from_slice(&self.rgba_data[src_idx..src_idx + 4]);
            }
        }

        Some(Self {
            rgba_data: data,
            width: new_w,
            height: new_h,
        })
    }
}

/// Image cache keyed by URL.
///
/// Tier: T2-C (κ lookup + π persistence + μ decode)
///
/// Caches decoded images to avoid re-decoding on every frame.
/// Limited by max entry count and max total bytes.
pub struct ImageCache {
    /// URL → decoded image mapping.
    entries: BTreeMap<String, CacheEntry>,
    /// Maximum number of cached images.
    max_entries: usize,
    /// Maximum total bytes across all cached images.
    max_bytes: usize,
    /// Current total bytes.
    current_bytes: usize,
    /// URLs that failed to load (avoid retry loops).
    failed: BTreeMap<String, u32>,
}

/// A cache entry with access metadata.
struct CacheEntry {
    image: DecodedImage,
    /// Number of times this entry was accessed.
    access_count: u64,
}

impl Default for ImageCache {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageCache {
    /// Create a new cache with default limits.
    ///
    /// Defaults: 128 entries, 256MB max.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            max_entries: 128,
            max_bytes: 256 * 1024 * 1024, // 256 MB
            current_bytes: 0,
            failed: BTreeMap::new(),
        }
    }

    /// Create with custom limits.
    #[must_use]
    pub fn with_limits(max_entries: usize, max_bytes: usize) -> Self {
        Self {
            entries: BTreeMap::new(),
            max_entries,
            max_bytes,
            current_bytes: 0,
            failed: BTreeMap::new(),
        }
    }

    /// Look up a cached image by URL.
    pub fn get(&mut self, url: &str) -> Option<&DecodedImage> {
        if let Some(entry) = self.entries.get_mut(url) {
            entry.access_count += 1;
            Some(&entry.image)
        } else {
            None
        }
    }

    /// Insert a decoded image into the cache.
    ///
    /// Evicts least-accessed entries if limits are exceeded.
    pub fn insert(&mut self, url: String, image: DecodedImage) {
        let image_bytes = image.byte_size();

        // Evict if necessary
        while (self.entries.len() >= self.max_entries
            || self.current_bytes + image_bytes > self.max_bytes)
            && !self.entries.is_empty()
        {
            self.evict_one();
        }

        self.current_bytes += image_bytes;
        self.entries.insert(
            url,
            CacheEntry {
                image,
                access_count: 1,
            },
        );
    }

    /// Record a failed URL to avoid retry storms.
    pub fn mark_failed(&mut self, url: &str) {
        let count = self.failed.entry(url.to_string()).or_insert(0);
        *count += 1;
    }

    /// Check if a URL is already cached (without incrementing access count).
    #[must_use]
    pub fn contains(&self, url: &str) -> bool {
        self.entries.contains_key(url)
    }

    /// Check if a URL has failed before.
    #[must_use]
    pub fn is_failed(&self, url: &str) -> bool {
        self.failed.contains_key(url)
    }

    /// Number of cached images.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Total bytes used by cached images.
    #[must_use]
    pub fn bytes_used(&self) -> usize {
        self.current_bytes
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.current_bytes = 0;
        self.failed.clear();
    }

    /// Evict the least-accessed entry.
    fn evict_one(&mut self) {
        if let Some(url) = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.access_count)
            .map(|(url, _)| url.clone())
        {
            if let Some(entry) = self.entries.remove(&url) {
                self.current_bytes = self.current_bytes.saturating_sub(entry.image.byte_size());
            }
        }
    }
}

/// Detect image format from bytes (magic number sniffing).
///
/// Returns the MIME type if recognized.
#[must_use]
pub fn detect_format(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 4 {
        return None;
    }
    match &bytes[..4] {
        [0x89, b'P', b'N', b'G'] => Some("image/png"),
        [0xFF, 0xD8, 0xFF, _] => Some("image/jpeg"),
        [b'R', b'I', b'F', b'F'] if bytes.len() >= 12 && &bytes[8..12] == b"WEBP" => {
            Some("image/webp")
        }
        [b'G', b'I', b'F', b'8'] => Some("image/gif"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_placeholder_image() {
        let img = DecodedImage::placeholder();
        assert_eq!(img.width, 1);
        assert_eq!(img.height, 1);
        assert_eq!(img.rgba_data.len(), 4);
        assert_eq!(img.pixel_count(), 1);
        // Magenta: R=255, G=0, B=255, A=255
        assert_eq!(&img.rgba_data, &[255, 0, 255, 255]);
    }

    #[test]
    fn test_decode_png() {
        // Minimal 1x1 red PNG (generated inline)
        let png_data = create_1x1_png(255, 0, 0, 255);
        let img = DecodedImage::decode(&png_data);
        assert!(img.is_some());
        let img = img.unwrap_or_else(DecodedImage::placeholder);
        assert_eq!(img.width, 1);
        assert_eq!(img.height, 1);
        assert_eq!(img.rgba_data[0], 255); // Red
        assert_eq!(img.rgba_data[1], 0); // Green
        assert_eq!(img.rgba_data[2], 0); // Blue
        assert_eq!(img.rgba_data[3], 255); // Alpha
    }

    #[test]
    fn test_decode_invalid_bytes() {
        let img = DecodedImage::decode(b"not an image");
        assert!(img.is_none());
    }

    #[test]
    fn test_decode_empty_bytes() {
        let img = DecodedImage::decode(b"");
        assert!(img.is_none());
    }

    #[test]
    fn test_cache_insert_and_get() {
        let mut cache = ImageCache::new();
        let img = DecodedImage::placeholder();

        cache.insert("https://example.com/logo.png".to_string(), img);
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());

        let retrieved = cache.get("https://example.com/logo.png");
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = ImageCache::new();
        assert!(cache.get("https://example.com/missing.png").is_none());
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = ImageCache::with_limits(2, 1024 * 1024);

        cache.insert("a.png".to_string(), DecodedImage::placeholder());
        cache.insert("b.png".to_string(), DecodedImage::placeholder());
        assert_eq!(cache.len(), 2);

        // Third insert should evict one
        cache.insert("c.png".to_string(), DecodedImage::placeholder());
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = ImageCache::new();
        cache.insert("a.png".to_string(), DecodedImage::placeholder());
        cache.insert("b.png".to_string(), DecodedImage::placeholder());
        cache.clear();
        assert!(cache.is_empty());
        assert_eq!(cache.bytes_used(), 0);
    }

    #[test]
    fn test_failed_url_tracking() {
        let mut cache = ImageCache::new();
        assert!(!cache.is_failed("bad.png"));

        cache.mark_failed("bad.png");
        assert!(cache.is_failed("bad.png"));
    }

    #[test]
    fn test_contains_without_access_bump() {
        let mut cache = ImageCache::new();
        assert!(!cache.contains("test.png"));

        cache.insert("test.png".to_string(), DecodedImage::placeholder());
        assert!(cache.contains("test.png"));
        assert!(!cache.contains("other.png"));
    }

    #[test]
    fn test_scale_to_fit_no_change() {
        let img = DecodedImage {
            rgba_data: vec![0; 100 * 100 * 4],
            width: 100,
            height: 100,
        };
        // Already fits
        assert!(img.scale_to_fit(200, 200).is_none());
    }

    #[test]
    fn test_scale_to_fit_downscale() {
        let img = DecodedImage {
            rgba_data: vec![128; 200 * 100 * 4],
            width: 200,
            height: 100,
        };
        let scaled = img.scale_to_fit(100, 100);
        assert!(scaled.is_some());
        let s = scaled.unwrap_or_else(DecodedImage::placeholder);
        assert_eq!(s.width, 100);
        assert_eq!(s.height, 50); // Maintains 2:1 aspect ratio
    }

    #[test]
    fn test_detect_format_png() {
        assert_eq!(
            detect_format(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]),
            Some("image/png")
        );
    }

    #[test]
    fn test_detect_format_jpeg() {
        assert_eq!(detect_format(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("image/jpeg"));
    }

    #[test]
    fn test_detect_format_gif() {
        assert_eq!(detect_format(b"GIF89a"), Some("image/gif"));
    }

    #[test]
    fn test_detect_format_unknown() {
        assert_eq!(detect_format(b"????"), None);
    }

    #[test]
    fn test_detect_format_too_short() {
        assert_eq!(detect_format(b"ab"), None);
    }

    #[test]
    fn test_byte_size() {
        let img = DecodedImage {
            rgba_data: vec![0; 10 * 10 * 4],
            width: 10,
            height: 10,
        };
        assert_eq!(img.byte_size(), 400);
        assert_eq!(img.pixel_count(), 100);
    }

    /// Create a minimal 1x1 PNG image for testing.
    fn create_1x1_png(r: u8, g: u8, b: u8, a: u8) -> Vec<u8> {
        use std::io::Write;
        let mut buf = Vec::new();

        // PNG signature
        buf.write_all(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A])
            .ok();

        // IHDR chunk
        let ihdr_data = [
            0, 0, 0, 1, // width=1
            0, 0, 0, 1, // height=1
            8, // bit depth
            6, // color type (RGBA)
            0, // compression
            0, // filter
            0, // interlace
        ];
        write_png_chunk(&mut buf, b"IHDR", &ihdr_data);

        // IDAT chunk (zlib-compressed scanline: filter=0, then RGBA)
        let raw_scanline = [0u8, r, g, b, a]; // filter byte + pixel
        let compressed = miniz_compress(&raw_scanline);
        write_png_chunk(&mut buf, b"IDAT", &compressed);

        // IEND chunk
        write_png_chunk(&mut buf, b"IEND", &[]);

        buf
    }

    fn write_png_chunk(buf: &mut Vec<u8>, chunk_type: &[u8; 4], data: &[u8]) {
        use std::io::Write;
        let len = data.len() as u32;
        buf.write_all(&len.to_be_bytes()).ok();
        buf.write_all(chunk_type).ok();
        buf.write_all(data).ok();
        // CRC32 over type + data
        let mut crc_data = Vec::with_capacity(4 + data.len());
        crc_data.extend_from_slice(chunk_type);
        crc_data.extend_from_slice(data);
        let crc = crc32(&crc_data);
        buf.write_all(&crc.to_be_bytes()).ok();
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc: u32 = 0xFFFF_FFFF;
        for &byte in data {
            crc ^= u32::from(byte);
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

    /// Minimal zlib/deflate compression (store only, no actual compression).
    fn miniz_compress(data: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        // zlib header (CMF=0x78, FLG=0x01 — no dict, check bits)
        out.push(0x78);
        out.push(0x01);
        // deflate: final block, stored (BTYPE=00)
        out.push(0x01); // BFINAL=1, BTYPE=00
        let len = data.len() as u16;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&(!len).to_le_bytes());
        out.extend_from_slice(data);
        // Adler32 checksum
        let adler = adler32(data);
        out.extend_from_slice(&adler.to_be_bytes());
        out
    }

    fn adler32(data: &[u8]) -> u32 {
        let mut a: u32 = 1;
        let mut b: u32 = 0;
        for &byte in data {
            a = (a + u32::from(byte)) % 65521;
            b = (b + a) % 65521;
        }
        (b << 16) | a
    }
}
