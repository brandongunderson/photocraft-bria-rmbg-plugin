use crate::protocol::{Job, Output, dimensions};
use image::RgbaImage;
use ort::{
    session::{RunOptions, Session, builder::GraphOptimizationLevel},
    value::Tensor,
};
use std::{
    fs::File,
    io::{BufReader, BufWriter},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

pub const SIDE: usize = 1024;
type Result<T> = std::result::Result<T, String>;

pub fn check_cancel(cancel: &AtomicBool) -> Result<()> {
    if cancel.load(Ordering::Acquire) {
        Err("Cancelled.".into())
    } else {
        Ok(())
    }
}

/// Windows' system loader and ONNX's file APIs do not inherit Rust's automatic
/// long-path handling. canonicalize preserves the native extended-length prefix
/// (including UNC paths) before either third-party library sees the path.
fn asset_path(path: &Path, label: &str) -> Result<std::path::PathBuf> {
    std::fs::canonicalize(path).map_err(|error| format!("Cannot resolve bundled {label}: {error}"))
}

fn load_failure(payload: Box<dyn std::any::Any + Send>) -> String {
    let detail = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("unknown native loader failure");
    format!(
        "Cannot load bundled ONNX Runtime: {}",
        detail.chars().take(1500).collect::<String>()
    )
}

pub fn read_source(path: &Path) -> Result<RgbaImage> {
    let mut decoder =
        png::Decoder::new(BufReader::new(File::open(path).map_err(|e| e.to_string())?));
    decoder.set_limits(png::Limits {
        bytes: 256 * 1024 * 1024,
    });
    let mut reader = decoder.read_info().map_err(|e| e.to_string())?;
    let info = reader.info();
    dimensions(info.width, info.height)?;
    if info.bit_depth != png::BitDepth::Eight
        || info.color_type != png::ColorType::Rgba
        || info.animation_control.is_some()
    {
        return Err("Expected one 8-bit RGBA PNG from PhotoCraft.".into());
    }
    let (width, height) = (info.width, info.height);
    let mut bytes = vec![0; width as usize * height as usize * 4];
    reader.next_frame(&mut bytes).map_err(|e| e.to_string())?;
    RgbaImage::from_raw(width, height, bytes).ok_or_else(|| "Invalid PNG buffer.".into())
}

/// PyTorch bilinear interpolation with align_corners=false, including clamped borders.
fn coordinate(
    destination: usize,
    source_size: usize,
    destination_size: usize,
) -> (usize, usize, f32) {
    let position =
        ((destination as f32 + 0.5) * source_size as f32 / destination_size as f32 - 0.5).max(0.0);
    let low = (position.floor() as usize).min(source_size - 1);
    (
        low,
        (low + 1).min(source_size - 1),
        position - position.floor(),
    )
}

fn interpolate(a: f32, b: f32, c: f32, d: f32, tx: f32, ty: f32) -> f32 {
    (a * (1.0 - tx) + b * tx) * (1.0 - ty) + (c * (1.0 - tx) + d * tx) * ty
}

/// BRIA uses RGB / 255 - 0.5, NCHW float32, with no second sigmoid.
pub fn preprocess(source: &RgbaImage, cancel: &AtomicBool) -> Result<Vec<f32>> {
    dimensions(source.width(), source.height())?;
    let mut tensor = vec![0.0; SIDE * SIDE * 3];
    for y in 0..SIDE {
        check_cancel(cancel)?;
        let (y0, y1, ty) = coordinate(y, source.height() as usize, SIDE);
        for x in 0..SIDE {
            let (x0, x1, tx) = coordinate(x, source.width() as usize, SIDE);
            for channel in 0..3 {
                let sample = |x, y| f32::from(source.get_pixel(x as u32, y as u32)[channel]);
                tensor[channel * SIDE * SIDE + y * SIDE + x] = interpolate(
                    sample(x0, y0),
                    sample(x1, y0),
                    sample(x0, y1),
                    sample(x1, y1),
                    tx,
                    ty,
                ) / 255.0
                    - 0.5;
            }
        }
    }
    Ok(tensor)
}

/// Resize floating mask first, then apply BRIA's global min/max normalization.
pub fn compose(
    source: &RgbaImage,
    values: &[f32],
    output: Output,
    cancel: &AtomicBool,
) -> Result<RgbaImage> {
    dimensions(source.width(), source.height())?;
    if values.len() != SIDE * SIDE || values.iter().any(|v| !v.is_finite()) {
        return Err("BRIA returned an invalid or non-finite mask.".into());
    }
    let width = source.width() as usize;
    let height = source.height() as usize;
    let mut resized = vec![0.0f32; width * height];
    let (mut minimum, mut maximum) = (f32::INFINITY, f32::NEG_INFINITY);
    for y in 0..height {
        check_cancel(cancel)?;
        let (y0, y1, ty) = coordinate(y, SIDE, height);
        for x in 0..width {
            let (x0, x1, tx) = coordinate(x, SIDE, width);
            let value = interpolate(
                values[y0 * SIDE + x0],
                values[y0 * SIDE + x1],
                values[y1 * SIDE + x0],
                values[y1 * SIDE + x1],
                tx,
                ty,
            );
            resized[y * width + x] = value;
            minimum = minimum.min(value);
            maximum = maximum.max(value);
        }
    }
    let span = maximum - minimum;
    let mut result = source.clone();
    for (index, (pixel, value)) in result.pixels_mut().zip(resized).enumerate() {
        if index % 65536 == 0 {
            check_cancel(cancel)?;
        }
        // The reference divides by zero for constant predictions. Keep their confidence instead.
        let mask = if span > f32::EPSILON {
            (value - minimum) / span
        } else {
            value
        };
        let mask = (mask.clamp(0.0, 1.0) * 255.0) as u16;
        let alpha = u16::from(pixel[3]);
        match output {
            Output::Foreground => pixel[3] = ((alpha * mask + 127) / 255) as u8,
            Output::Background => pixel[3] = ((alpha * (255 - mask) + 127) / 255) as u8,
            Output::Mask => {
                let coverage = ((alpha * mask + 127) / 255) as u8;
                *pixel = image::Rgba([coverage, coverage, coverage, 255]);
            }
        }
    }
    Ok(result)
}

#[derive(Default)]
pub struct Engine {
    session: Option<Session>,
}

impl Engine {
    pub fn process(
        &mut self,
        job: &Job,
        runtime: &Path,
        cancel: &AtomicBool,
        progress: &dyn Fn(f64, &str),
    ) -> Result<()> {
        check_cancel(cancel)?;
        progress(0.02, "Reading image");
        let source = read_source(&job.input)?;
        if source.dimensions() != (job.output_size.width, job.output_size.height) {
            return Err("BRIA preserves the source dimensions; output size must match.".into());
        }
        progress(0.08, "Preparing image");
        let pixels = preprocess(&source, cancel)?;
        if self.session.is_none() {
            progress(0.15, "Loading local BRIA model");
            let dll = runtime.join("bin/onnxruntime.dll");
            let model = runtime.join("models/rmbg-1.4.onnx");
            if !dll.is_file() || !model.is_file() {
                return Err(
                    "Bundled BRIA model or runtime is missing. Reinstall the complete plugin."
                        .into(),
                );
            }
            let dll = asset_path(&dll, "ONNX Runtime")?;
            let model = asset_path(&model, "BRIA model")?;
            std::panic::catch_unwind(|| {
                ort::init_from(dll.to_string_lossy())
                    .with_name("BRIA RMBG")
                    .with_telemetry(false)
                    .commit()
            })
            .map_err(load_failure)?
            .map_err(|e| e.to_string())?;
            let session = Session::builder()
                .map_err(|e| e.to_string())?
                .with_log_level(ort::logging::LogLevel::Error)
                .map_err(|e| e.to_string())?
                .with_optimization_level(GraphOptimizationLevel::Level1)
                .map_err(|e| e.to_string())?
                .with_intra_threads(4)
                .map_err(|e| e.to_string())?
                .with_inter_threads(1)
                .map_err(|e| e.to_string())?
                .with_parallel_execution(false)
                .map_err(|e| e.to_string())?
                .commit_from_file(model)
                .map_err(|e| e.to_string())?;
            if session.inputs.len() != 1 || session.outputs.is_empty() {
                return Err("Incompatible BRIA model input/output signature.".into());
            }
            self.session = Some(session);
        }
        check_cancel(cancel)?;
        progress(0.30, "Separating subject and background");
        let tensor = Tensor::from_array(([1usize, 3, SIDE, SIDE], pixels.into_boxed_slice()))
            .map_err(|e| e.to_string())?;
        let session = self.session.as_mut().ok_or("BRIA model did not load.")?;
        let options = RunOptions::new().map_err(|e| e.to_string())?;
        let finished = AtomicBool::new(false);
        let outputs = std::thread::scope(|scope| {
            scope.spawn(|| {
                while !finished.load(Ordering::Acquire) {
                    if cancel.load(Ordering::Acquire) {
                        let _ = options.terminate();
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(25));
                }
            });
            let result = session.run_with_options(ort::inputs![tensor], &options);
            finished.store(true, Ordering::Release);
            result
        });
        check_cancel(cancel)?;
        let outputs = outputs.map_err(|e| e.to_string())?;
        let first = outputs.values().next().ok_or("BRIA returned no mask.")?;
        let (shape, values) = first
            .try_extract_tensor::<f32>()
            .map_err(|e| e.to_string())?;
        if shape.as_ref() != [1, 1, SIDE as i64, SIDE as i64] {
            return Err("BRIA returned an unexpected mask shape.".into());
        }
        progress(0.85, "Restoring original dimensions");
        let image = compose(&source, values, job.options.output, cancel)?;
        drop(outputs);
        check_cancel(cancel)?;
        let parent = job.output.parent().ok_or("Missing output directory.")?;
        let temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        {
            let mut encoder = png::Encoder::new(
                BufWriter::new(temporary.as_file()),
                image.width(),
                image.height(),
            );
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            encoder.set_source_srgb(png::SrgbRenderingIntent::Perceptual);
            let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
            writer
                .write_image_data(image.as_raw())
                .map_err(|e| e.to_string())?;
            writer.finish().map_err(|e| e.to_string())?;
        }
        check_cancel(cancel)?;
        temporary
            .persist_noclobber(&job.output)
            .map_err(|e| e.to_string())?;
        if let Err(error) = check_cancel(cancel) {
            let _ = std::fs::remove_file(&job.output);
            return Err(error);
        }
        progress(1.0, "Result ready");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    static CLEAR: AtomicBool = AtomicBool::new(false);

    #[test]
    fn preprocessing_uses_bria_range_and_rgb_plane_order() {
        let source = RgbaImage::from_pixel(1, 1, image::Rgba([0, 128, 255, 0]));
        let input = preprocess(&source, &CLEAR).unwrap();
        assert_eq!(input[0], -0.5);
        assert!((input[SIDE * SIDE] - (128.0 / 255.0 - 0.5)).abs() < 1e-6);
        assert_eq!(input[2 * SIDE * SIDE], 0.5);
    }

    #[test]
    fn full_size_masks_preserve_color_and_multiply_existing_alpha() {
        let source = RgbaImage::from_raw(2, 1, vec![11, 22, 33, 128, 44, 55, 66, 0]).unwrap();
        let values = vec![0.5; SIDE * SIDE];
        let foreground = compose(&source, &values, Output::Foreground, &CLEAR).unwrap();
        assert_eq!(foreground.as_raw(), &[11, 22, 33, 64, 44, 55, 66, 0]);
        let background = compose(&source, &values, Output::Background, &CLEAR).unwrap();
        assert_eq!(background.get_pixel(0, 0)[3], 64);
        let mask = compose(&source, &values, Output::Mask, &CLEAR).unwrap();
        assert_eq!(mask.as_raw(), &[64, 64, 64, 255, 0, 0, 0, 255]);
    }

    #[test]
    fn minmax_normalization_and_constant_masks_are_defined() {
        let source = RgbaImage::from_pixel(SIDE as u32, 1, image::Rgba([1, 2, 3, 255]));
        let values: Vec<f32> = (0..SIDE * SIDE)
            .map(|i| 0.2 + 0.4 * (i % SIDE) as f32 / (SIDE - 1) as f32)
            .collect();
        let result = compose(&source, &values, Output::Foreground, &CLEAR).unwrap();
        assert_eq!(result.get_pixel(0, 0)[3], 0);
        assert_eq!(result.get_pixel(SIDE as u32 - 1, 0)[3], 255);
        for value in [0.0, 1.0] {
            let result = compose(
                &source,
                &vec![value; SIDE * SIDE],
                Output::Foreground,
                &CLEAR,
            )
            .unwrap();
            assert_eq!(result.get_pixel(0, 0)[3], (value * 255.0) as u8);
        }
    }

    #[test]
    fn invalid_masks_and_cancellation_fail() {
        let source = RgbaImage::from_pixel(1, 1, image::Rgba([1, 2, 3, 255]));
        assert!(compose(&source, &[], Output::Mask, &CLEAR).is_err());
        assert!(compose(&source, &vec![f32::NAN; SIDE * SIDE], Output::Mask, &CLEAR).is_err());
        assert!(preprocess(&source, &AtomicBool::new(true)).is_err());
        assert!(
            compose(
                &source,
                &vec![0.0; SIDE * SIDE],
                Output::Mask,
                &AtomicBool::new(true)
            )
            .is_err()
        );
    }
}
