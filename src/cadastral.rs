// Cadastral data fetcher — geocode a Korean address via VWorld, then download
// cadastral boundary polygons and satellite/cadastral imagery.
//
// Native builds hit the VWorld API directly; WASM builds go through the
// same-origin proxy at /api/vworld/*.

use std::f64::consts::PI;

const KEY: &str = "2B8947EA-9882-40CF-8217-71ECA7ED3CA2";
const WEB_MERCATOR_EXTENT: f64 = 40_075_016.685_578_488;
const ZOOM: u32 = 18;
const GRID: u32 = 4;

/// A single parcel boundary ring (closed polyline in EPSG:3857).
#[derive(Debug, Clone)]
pub struct ParcelRing {
    pub points: Vec<(f64, f64)>,
}

/// Everything the CADASTRAL command needs to add to the drawing.
#[derive(Debug, Clone)]
pub struct CadastralData {
    pub parcels: Vec<ParcelRing>,
    /// EPSG:3857 bounding box: (west, south, east, north).
    pub bbox: (f64, f64, f64, f64),
    /// Stitched satellite image as PNG bytes.
    pub satellite_png: Vec<u8>,
    pub satellite_width: u32,
    pub satellite_height: u32,
    /// Cadastral map image as PNG bytes.
    pub cadastral_png: Vec<u8>,
    pub cadastral_width: u32,
    pub cadastral_height: u32,
}

fn tile_of(lon: f64, lat: f64, z: u32) -> (u32, u32) {
    let n = (1u64 << z) as f64;
    let x = ((lon + 180.0) / 360.0 * n) as u32;
    let lat_r = lat.to_radians();
    let y = ((1.0 - (lat_r.tan() + 1.0 / lat_r.cos()).ln() / PI) / 2.0 * n) as u32;
    (x, y)
}

fn tile_bounds_3857(x: u32, y: u32, z: u32) -> (f64, f64, f64, f64) {
    let n = (1u64 << z) as f64;
    let size = WEB_MERCATOR_EXTENT / n;
    let origin = -WEB_MERCATOR_EXTENT / 2.0;
    let min_x = origin + x as f64 * size;
    let max_y = -origin - y as f64 * size;
    (min_x, max_y - size, min_x + size, max_y)
}

/// Percent-encode a string for use in a URL query parameter.
fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for byte in input.as_bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            _ => {
                out.push('%');
                out.push(char::from(b"0123456789ABCDEF"[(byte >> 4) as usize]));
                out.push(char::from(b"0123456789ABCDEF"[(byte & 0x0F) as usize]));
            }
        }
    }
    out
}

// ── Native implementation ──────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub async fn fetch_cadastral(address: String) -> Result<CadastralData, String> {
    let agent = crate::network::agent(std::time::Duration::from_secs(30));

    let (lon, lat) = geocode_native(&agent, &address)?;
    let (cx, cy) = tile_of(lon, lat, ZOOM);

    let (satellite_png, satellite_width, satellite_height, bbox) =
        fetch_satellite_native(&agent, cx, cy, ZOOM, GRID)?;
    let cadastral_png_data = fetch_cadastral_image_native(&agent, &bbox)?;
    let parcels = fetch_parcels_native(&agent, &bbox)?;

    let cadastral_img = image::load_from_memory(&cadastral_png_data)
        .map_err(|e| format!("cadastral image decode: {e}"))?;
    let (cw, ch) = image::GenericImageView::dimensions(&cadastral_img);

    Ok(CadastralData {
        parcels,
        bbox,
        satellite_png,
        satellite_width,
        satellite_height,
        cadastral_png: cadastral_png_data,
        cadastral_width: cw,
        cadastral_height: ch,
    })
}

#[cfg(not(target_arch = "wasm32"))]
fn geocode_native(agent: &ureq::Agent, address: &str) -> Result<(f64, f64), String> {
    for addr_type in &["road", "parcel"] {
        let url = format!(
            "https://api.vworld.kr/req/address?service=address&request=getcoord&version=2.0\
             &crs=epsg:4326&address={}&format=json&type={}&key={}",
            percent_encode(address),
            addr_type,
            KEY,
        );
        let text = agent
            .get(&url)
            .call()
            .map_err(|e| format!("geocode: {e}"))?
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("geocode read: {e}"))?;
        let body: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("geocode JSON: {e}"))?;
        if body["response"]["status"].as_str() == Some("OK") {
            let pt = &body["response"]["result"]["point"];
            let x = pt["x"]
                .as_str()
                .and_then(|s: &str| s.parse::<f64>().ok())
                .ok_or("geocode: missing x")?;
            let y = pt["y"]
                .as_str()
                .and_then(|s: &str| s.parse::<f64>().ok())
                .ok_or("geocode: missing y")?;
            return Ok((x, y));
        }
    }
    Err(format!("Geocoding failed for '{address}'"))
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_satellite_native(
    agent: &ureq::Agent,
    cx: u32,
    cy: u32,
    z: u32,
    grid: u32,
) -> Result<(Vec<u8>, u32, u32, (f64, f64, f64, f64)), String> {
    let half = grid / 2;
    let x0 = cx - half;
    let y0 = cy - half;
    let tile_px = 256u32;
    let total = grid * tile_px;
    let mut canvas = image::RgbImage::new(total, total);

    for dy in 0..grid {
        for dx in 0..grid {
            let url = format!(
                "https://api.vworld.kr/req/wmts/1.0.0/{}/Satellite/{}/{}/{}.jpeg",
                KEY,
                z,
                y0 + dy,
                x0 + dx,
            );
            let bytes: Vec<u8> = agent
                .get(&url)
                .call()
                .map_err(|e| format!("satellite tile: {e}"))?
                .body_mut()
                .read_to_vec()
                .map_err(|e| format!("satellite tile read: {e}"))?;
            let tile = image::load_from_memory(&bytes)
                .map_err(|e| format!("satellite tile decode: {e}"))?
                .to_rgb8();
            image::imageops::overlay(
                &mut canvas,
                &tile,
                (dx * tile_px) as i64,
                (dy * tile_px) as i64,
            );
        }
    }

    let (west, _, _, north) = tile_bounds_3857(x0, y0, z);
    let (_, south, east, _) = tile_bounds_3857(x0 + grid - 1, y0 + grid - 1, z);

    let mut png_bytes = std::io::Cursor::new(Vec::new());
    canvas
        .write_to(&mut png_bytes, image::ImageFormat::Png)
        .map_err(|e| format!("satellite PNG encode: {e}"))?;

    Ok((png_bytes.into_inner(), total, total, (west, south, east, north)))
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_cadastral_image_native(
    agent: &ureq::Agent,
    bbox: &(f64, f64, f64, f64),
) -> Result<Vec<u8>, String> {
    let url = format!(
        "https://api.vworld.kr/req/image?service=image&request=getmap\
         &basemap=GRAPHIC&layers=lp_pa_cbnd_bonbun\
         &bbox={},{},{},{}&crs=EPSG:3857&width=1024&height=1024\
         &format=png&transparent=true&key={}",
        bbox.0, bbox.1, bbox.2, bbox.3, KEY,
    );
    let bytes: Vec<u8> = agent
        .get(&url)
        .call()
        .map_err(|e| format!("cadastral image: {e}"))?
        .body_mut()
        .read_to_vec()
        .map_err(|e| format!("cadastral image read: {e}"))?;
    Ok(bytes)
}

#[cfg(not(target_arch = "wasm32"))]
fn fetch_parcels_native(
    agent: &ureq::Agent,
    bbox: &(f64, f64, f64, f64),
) -> Result<Vec<ParcelRing>, String> {
    let mut rings = Vec::new();
    let mut page = 1u32;
    loop {
        let url = format!(
            "https://api.vworld.kr/req/data?service=data&request=GetFeature\
             &data=LP_PA_CBND_BUBUN&key={}&format=json&crs=EPSG:3857\
             &geomFilter=BOX({},{},{},{})&size=1000&page={}&domain=opencad.dnaboy.org",
            KEY, bbox.0, bbox.1, bbox.2, bbox.3, page,
        );
        let text = agent
            .get(&url)
            .call()
            .map_err(|e| format!("parcels: {e}"))?
            .body_mut()
            .read_to_string()
            .map_err(|e| format!("parcels read: {e}"))?;
        let body: serde_json::Value =
            serde_json::from_str(&text).map_err(|e| format!("parcels JSON: {e}"))?;
        let resp = &body["response"];
        if resp["status"].as_str() != Some("OK") {
            if page == 1 {
                return Err("Cadastral parcel fetch failed".into());
            }
            break;
        }
        if let Some(features) = resp["result"]["featureCollection"]["features"].as_array() {
            for feature in features {
                extract_rings(&feature["geometry"], &mut rings);
            }
        }
        let total_pages = resp["page"]["total"]
            .as_str()
            .and_then(|s: &str| s.parse::<u32>().ok())
            .unwrap_or(1);
        if page >= total_pages {
            break;
        }
        page += 1;
    }
    Ok(rings)
}

fn extract_rings(geometry: &serde_json::Value, out: &mut Vec<ParcelRing>) {
    let gtype = geometry["type"].as_str().unwrap_or("");
    let coords = &geometry["coordinates"];
    match gtype {
        "Polygon" => {
            if let Some(polygon) = coords.as_array() {
                for ring in polygon {
                    if let Some(ring) = parse_ring(ring) {
                        out.push(ring);
                    }
                }
            }
        }
        "MultiPolygon" => {
            if let Some(polys) = coords.as_array() {
                for poly in polys {
                    if let Some(polygon) = poly.as_array() {
                        for ring in polygon {
                            if let Some(ring) = parse_ring(ring) {
                                out.push(ring);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn parse_ring(ring: &serde_json::Value) -> Option<ParcelRing> {
    let coords = ring.as_array()?;
    let points: Vec<(f64, f64)> = coords
        .iter()
        .filter_map(|pt| {
            let arr = pt.as_array()?;
            Some((arr.first()?.as_f64()?, arr.get(1)?.as_f64()?))
        })
        .collect();
    if points.len() >= 3 {
        Some(ParcelRing { points })
    } else {
        None
    }
}

// ── WASM implementation ────────────────────────────────────────────────────

#[cfg(target_arch = "wasm32")]
pub async fn fetch_cadastral(address: String) -> Result<CadastralData, String> {
    let (lon, lat) = geocode_web(&address).await?;
    let (cx, cy) = tile_of(lon, lat, ZOOM);

    let (satellite_png, sw, sh, bbox) = fetch_satellite_web(cx, cy, ZOOM, GRID).await?;
    let cadastral_png_data = fetch_cadastral_image_web(&bbox).await?;
    let parcels = fetch_parcels_web(&bbox).await?;

    let cadastral_img = image::load_from_memory(&cadastral_png_data)
        .map_err(|e| format!("cadastral image decode: {e}"))?;
    let (cw, ch) = image::GenericImageView::dimensions(&cadastral_img);

    Ok(CadastralData {
        parcels,
        bbox,
        satellite_png,
        satellite_width: sw,
        satellite_height: sh,
        cadastral_png: cadastral_png_data,
        cadastral_width: cw,
        cadastral_height: ch,
    })
}

#[cfg(target_arch = "wasm32")]
async fn fetch_json_web(url: &str) -> Result<serde_json::Value, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or("no window")?;
    let response = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|_| format!("fetch failed: {url}"))?
        .dyn_into::<web_sys::Response>()
        .map_err(|_| "invalid Response")?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    let text = JsFuture::from(response.text().map_err(|_| "text() failed")?)
        .await
        .map_err(|_| "body read failed")?
        .as_string()
        .ok_or("body is not a string")?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[cfg(target_arch = "wasm32")]
async fn fetch_bytes_web(url: &str) -> Result<Vec<u8>, String> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;

    let window = web_sys::window().ok_or("no window")?;
    let response = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|_| format!("fetch failed: {url}"))?
        .dyn_into::<web_sys::Response>()
        .map_err(|_| "invalid Response")?;
    if !response.ok() {
        return Err(format!("HTTP {}", response.status()));
    }
    let buffer = JsFuture::from(
        response
            .array_buffer()
            .map_err(|_| "array_buffer() failed")?,
    )
    .await
    .map_err(|_| "buffer read failed")?;
    let uint8 = js_sys::Uint8Array::new(&buffer);
    Ok(uint8.to_vec())
}

#[cfg(target_arch = "wasm32")]
async fn geocode_web(address: &str) -> Result<(f64, f64), String> {
    for addr_type in &["road", "parcel"] {
        let url = format!(
            "/api/vworld/address?service=address&request=getcoord&version=2.0\
             &crs=epsg:4326&address={}&format=json&type={}",
            percent_encode(address),
            addr_type,
        );
        let body = fetch_json_web(&url).await?;
        if body["response"]["status"].as_str() == Some("OK") {
            let pt = &body["response"]["result"]["point"];
            let x = pt["x"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or("geocode: missing x")?;
            let y = pt["y"]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or("geocode: missing y")?;
            return Ok((x, y));
        }
    }
    Err(format!("Geocoding failed for '{address}'"))
}

#[cfg(target_arch = "wasm32")]
async fn fetch_satellite_web(
    cx: u32,
    cy: u32,
    z: u32,
    grid: u32,
) -> Result<(Vec<u8>, u32, u32, (f64, f64, f64, f64)), String> {
    let half = grid / 2;
    let x0 = cx - half;
    let y0 = cy - half;
    let tile_px = 256u32;
    let total = grid * tile_px;
    let mut canvas = image::RgbImage::new(total, total);

    for dy in 0..grid {
        for dx in 0..grid {
            let url = format!(
                "/api/vworld/wms?service=wmts&request=GetTile&version=1.0.0\
                 &layer=Satellite&style=default&tilematrixset=EPSG:3857\
                 &tilematrix={z}&tilerow={}&tilecol={}&format=image/jpeg",
                y0 + dy,
                x0 + dx,
            );
            let bytes = fetch_bytes_web(&url).await?;
            let tile = image::load_from_memory(&bytes)
                .map_err(|e| format!("satellite tile decode: {e}"))?
                .to_rgb8();
            image::imageops::overlay(
                &mut canvas,
                &tile,
                (dx * tile_px) as i64,
                (dy * tile_px) as i64,
            );
        }
    }

    let (west, _, _, north) = tile_bounds_3857(x0, y0, z);
    let (_, south, east, _) = tile_bounds_3857(x0 + grid - 1, y0 + grid - 1, z);

    let mut png_bytes = std::io::Cursor::new(Vec::new());
    canvas
        .write_to(&mut png_bytes, image::ImageFormat::Png)
        .map_err(|e| format!("satellite PNG encode: {e}"))?;

    Ok((png_bytes.into_inner(), total, total, (west, south, east, north)))
}

#[cfg(target_arch = "wasm32")]
async fn fetch_cadastral_image_web(
    bbox: &(f64, f64, f64, f64),
) -> Result<Vec<u8>, String> {
    let url = format!(
        "/api/vworld/image?service=image&request=getmap\
         &basemap=GRAPHIC&layers=lp_pa_cbnd_bonbun\
         &bbox={},{},{},{}&crs=EPSG:3857&width=1024&height=1024\
         &format=png&transparent=true",
        bbox.0, bbox.1, bbox.2, bbox.3,
    );
    fetch_bytes_web(&url).await
}

#[cfg(target_arch = "wasm32")]
async fn fetch_parcels_web(
    bbox: &(f64, f64, f64, f64),
) -> Result<Vec<ParcelRing>, String> {
    let mut rings = Vec::new();
    let mut page = 1u32;
    loop {
        let url = format!(
            "/api/vworld/data?service=data&request=GetFeature\
             &data=LP_PA_CBND_BUBUN&format=json&crs=EPSG:3857\
             &geomFilter=BOX({},{},{},{})&size=1000&page={}&domain=opencad.dnaboy.org",
            bbox.0, bbox.1, bbox.2, bbox.3, page,
        );
        let body = fetch_json_web(&url).await?;
        let resp = &body["response"];
        if resp["status"].as_str() != Some("OK") {
            if page == 1 {
                return Err("Cadastral parcel fetch failed".into());
            }
            break;
        }
        if let Some(features) = resp["result"]["featureCollection"]["features"].as_array() {
            for feature in features {
                extract_rings(&feature["geometry"], &mut rings);
            }
        }
        let total_pages = resp["page"]["total"]
            .as_str()
            .and_then(|s: &str| s.parse::<u32>().ok())
            .unwrap_or(1);
        if page >= total_pages {
            break;
        }
        page += 1;
    }
    Ok(rings)
}
