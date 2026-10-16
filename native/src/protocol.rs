use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum Output {
    #[default]
    Foreground,
    Background,
    Mask,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Options {
    #[serde(default)]
    pub output: Output,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Job {
    pub api_version: u32,
    pub input: PathBuf,
    pub output: PathBuf,
    pub options: Options,
    pub output_size: Size,
    pub color_space: String,
}

pub fn dimensions(width: u32, height: u32) -> Result<(), String> {
    if width == 0
        || height == 0
        || width > 32768
        || height > 32768
        || u64::from(width) * u64::from(height) > 32_000_000
    {
        return Err("Image must fit within 32 megapixels and 32,768 pixels per side.".into());
    }
    Ok(())
}

impl Job {
    pub fn parse(value: serde_json::Value) -> Result<Self, String> {
        let job: Self =
            serde_json::from_value(value).map_err(|e| format!("Invalid image request: {e}"))?;
        dimensions(job.output_size.width, job.output_size.height)?;
        if job.api_version != 1 || job.color_space != "srgb" {
            return Err("Expected image payload API 1 with sRGB pixels.".into());
        }
        if !job.input.is_absolute()
            || !job.output.is_absolute()
            || job.input == job.output
            || job.output.exists()
        {
            return Err("Use distinct absolute input/output paths and a new output file.".into());
        }
        Ok(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strict_job_options_and_bounds() {
        let root = std::env::temp_dir();
        let good = json!({"apiVersion":1,"input":root.join("bria-input.png"),"output":root.join("bria-new-output.png"),
            "colorSpace":"srgb","outputSize":{"width":10,"height":9},"options":{"output":"mask"}});
        assert_eq!(
            Job::parse(good.clone()).unwrap().options.output,
            Output::Mask
        );
        for (field, value) in [
            ("apiVersion", json!(2)),
            ("colorSpace", json!("cmyk")),
            ("input", json!("relative.png")),
            ("options", json!({"output":"invalid"})),
            ("options", json!({"unused":true})),
        ] {
            let mut bad = good.clone();
            bad[field] = value;
            assert!(Job::parse(bad).is_err());
        }
        for (w, h) in [
            (0, 1),
            (1, 0),
            (32769, 1),
            (10000, 10000),
            (u32::MAX, u32::MAX),
        ] {
            assert!(dimensions(w, h).is_err());
        }
    }
}
