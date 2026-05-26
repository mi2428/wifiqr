use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

mod version;

use anyhow::{Context, Result, bail};
use clap::{Args, Parser, Subcommand, ValueEnum};
use image::{Rgb, RgbImage};
use qrcode::{EcLevel, QrCode, types::Color};
use unicode_width::UnicodeWidthStr;

const DEFAULT_ASCII_CHAR: &str = "⬜";
const MIN_ASCII_CELL_WIDTH: usize = 2;

#[derive(Debug, Parser)]
#[command(
    name = "wifiqr",
    version,
    long_version = version::LONG_VERSION,
    propagate_version = true,
    about = "Generate Wi-Fi QR codes as PNG, SVG, or terminal text art."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    #[command(flatten)]
    output: OutputOptions,

    #[arg(short, long, help = "Wi-Fi SSID for the default Wi-Fi mode")]
    ssid: Option<String>,

    #[arg(short, long, help = "Wi-Fi password for WPA/WEP networks")]
    password: Option<String>,

    #[arg(
        short = 't',
        long,
        value_enum,
        default_value_t = Security::Wpa,
        help = "Wi-Fi security type"
    )]
    security: Security,

    #[arg(long, help = "Mark the Wi-Fi network as hidden in the QR payload")]
    hidden: bool,
}

impl Cli {
    fn request(&self) -> Result<QrRequest<'_>> {
        match &self.command {
            Some(Command::Raw { text, output }) => Ok(QrRequest {
                payload: text.clone(),
                output,
            }),
            None => {
                let ssid = self
                    .ssid
                    .as_deref()
                    .context("--ssid is required unless using the raw subcommand")?;
                let payload =
                    build_wifi_payload(ssid, self.password.as_deref(), self.security, self.hidden)?;

                Ok(QrRequest {
                    payload,
                    output: &self.output,
                })
            }
        }
    }
}

struct QrRequest<'a> {
    payload: String,
    output: &'a OutputOptions,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "Generate a QR code from an arbitrary raw string")]
    Raw {
        #[arg(help = "Raw text to encode")]
        text: String,

        #[command(flatten)]
        output: OutputOptions,
    },
}

#[derive(Clone, Debug, Args)]
struct OutputOptions {
    #[arg(short, long, help = "Output file path for PNG or SVG")]
    output: Option<PathBuf>,

    #[arg(
        short,
        long,
        value_enum,
        help = "Output format. Inferred from --output extension when omitted; defaults to ascii without --output"
    )]
    format: Option<OutputFormat>,

    #[arg(long, default_value_t = 1024, help = "PNG/SVG canvas size in pixels")]
    size: u32,

    #[arg(long, default_value_t = 4, help = "Quiet-zone border in QR modules")]
    border: usize,

    #[arg(
        short = 'e',
        long = "error-correction",
        value_enum,
        default_value_t = ErrorCorrection::H,
        help = "QR error correction level"
    )]
    error_correction: ErrorCorrection,

    #[arg(
        long = "ascii-char",
        default_value = DEFAULT_ASCII_CHAR,
        help = "Text used for dark modules in terminal text output"
    )]
    ascii_char: String,
}

impl OutputOptions {
    fn target(&self) -> Result<OutputTarget<'_>> {
        match self.format()? {
            OutputFormat::Png => Ok(OutputTarget::Png(self.required_output_path("PNG")?)),
            OutputFormat::Svg => Ok(OutputTarget::Svg(self.required_output_path("SVG")?)),
            OutputFormat::Ascii => {
                if self.output.is_some() {
                    bail!("terminal text output is printed to stdout; omit --output");
                }
                Ok(OutputTarget::Ascii)
            }
        }
    }

    fn format(&self) -> Result<OutputFormat> {
        if let Some(format) = self.format {
            return Ok(format);
        }

        let Some(path) = &self.output else {
            return Ok(OutputFormat::Ascii);
        };

        match path
            .extension()
            .and_then(|extension| extension.to_str())
            .map(str::to_ascii_lowercase)
            .as_deref()
        {
            Some("png") => Ok(OutputFormat::Png),
            Some("svg") => Ok(OutputFormat::Svg),
            _ => bail!("could not infer output format from extension; use --format png|svg|ascii"),
        }
    }

    fn required_output_path(&self, format: &str) -> Result<&Path> {
        self.output
            .as_deref()
            .with_context(|| format!("--output is required for {format} output"))
    }
}

#[derive(Debug)]
enum OutputTarget<'a> {
    Png(&'a Path),
    Svg(&'a Path),
    Ascii,
}

#[derive(Debug)]
struct AsciiStyle {
    dark: String,
    light: String,
}

impl AsciiStyle {
    fn new(dark: &str) -> Result<Self> {
        if dark.is_empty() {
            bail!("--ascii-char must not be empty");
        }
        if dark.chars().any(char::is_control) {
            bail!("--ascii-char must not contain control characters");
        }
        if dark.chars().all(char::is_whitespace) {
            bail!("--ascii-char must contain a visible character");
        }

        let width = UnicodeWidthStr::width(dark);
        if width == 0 {
            bail!("--ascii-char must have visible width");
        }

        let dark = if width < MIN_ASCII_CELL_WIDTH {
            dark.repeat(MIN_ASCII_CELL_WIDTH)
        } else {
            dark.to_string()
        };
        let light = " ".repeat(UnicodeWidthStr::width(dark.as_str()));

        Ok(Self { dark, light })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum OutputFormat {
    Png,
    Svg,
    Ascii,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ErrorCorrection {
    L,
    M,
    Q,
    H,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum Security {
    Wpa,
    Wep,
    #[value(name = "nopass", alias = "open")]
    NoPass,
}

#[derive(Debug)]
struct QrMatrix {
    width: usize,
    modules: Vec<bool>,
}

impl QrMatrix {
    fn encode(payload: &str, error_correction: ErrorCorrection, border: usize) -> Result<Self> {
        let code =
            QrCode::with_error_correction_level(payload.as_bytes(), error_correction.ec_level())
                .context("failed to generate QR code")?;
        Self::from_code(&code, border)
    }

    fn from_code(code: &QrCode, border: usize) -> Result<Self> {
        let inner_width = code.width();
        let padding = border.checked_mul(2).context("QR dimensions overflowed")?;
        let width = inner_width
            .checked_add(padding)
            .context("QR dimensions overflowed")?;
        let area = width
            .checked_mul(width)
            .context("QR dimensions overflowed")?;
        let mut modules = vec![false; area];
        let colors = code.to_colors();

        for y in 0..inner_width {
            for x in 0..inner_width {
                let source_idx = y * inner_width + x;
                let target_x = x + border;
                let target_y = y + border;
                modules[target_y * width + target_x] = colors[source_idx] == Color::Dark;
            }
        }

        Ok(Self { width, modules })
    }

    fn is_dark(&self, x: usize, y: usize) -> bool {
        self.modules[y * self.width + x]
    }

    fn dark_modules(&self) -> impl Iterator<Item = (usize, usize)> + '_ {
        (0..self.width).flat_map(move |y| {
            (0..self.width).filter_map(move |x| self.is_dark(x, y).then_some((x, y)))
        })
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let request = cli.request()?;
    let matrix = QrMatrix::encode(
        &request.payload,
        request.output.error_correction,
        request.output.border,
    )?;

    write_output(&matrix, request.output)
}

fn build_wifi_payload(
    ssid: &str,
    password: Option<&str>,
    security: Security,
    hidden: bool,
) -> Result<String> {
    let mut payload = format!(
        "WIFI:S:{};T:{};",
        escape_wifi_field(ssid),
        security.qr_value()
    );

    if security.requires_password() {
        let password =
            password.context("--password is required unless --security nopass is used")?;
        payload.push_str("P:");
        payload.push_str(&escape_wifi_field(password));
        payload.push(';');
    }

    if hidden {
        payload.push_str("H:true;");
    }

    payload.push(';');
    Ok(payload)
}

fn escape_wifi_field(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        if matches!(ch, '\\' | ';' | ',' | ':' | '"') {
            escaped.push('\\');
        }
        escaped.push(ch);
    }
    escaped
}

fn write_output(matrix: &QrMatrix, options: &OutputOptions) -> Result<()> {
    match options.target()? {
        OutputTarget::Png(path) => write_png(matrix, options.size, path),
        OutputTarget::Svg(path) => write_svg(matrix, options.size, path),
        OutputTarget::Ascii => {
            print!("{}", render_ascii(matrix, &options.ascii_char)?);
            Ok(())
        }
    }
}

fn write_png(matrix: &QrMatrix, size: u32, path: &Path) -> Result<()> {
    ensure_canvas_size(size)?;

    let module_count = u32::try_from(matrix.width).context("QR dimensions exceed PNG limits")?;
    let pixels_per_module = size / module_count;
    if pixels_per_module == 0 {
        bail!(
            "--size {} is too small for a QR code with {} modules",
            size,
            module_count
        );
    }

    let qr_pixels = pixels_per_module * module_count;
    let offset = (size - qr_pixels) / 2;
    let mut image = RgbImage::from_pixel(size, size, Rgb([255, 255, 255]));

    for (x, y) in matrix.dark_modules() {
        let x0 = offset + x as u32 * pixels_per_module;
        let y0 = offset + y as u32 * pixels_per_module;
        for py in y0..(y0 + pixels_per_module) {
            for px in x0..(x0 + pixels_per_module) {
                image.put_pixel(px, py, Rgb([0, 0, 0]));
            }
        }
    }

    image
        .save(path)
        .with_context(|| format!("failed to write PNG to {}", path.display()))
}

fn write_svg(matrix: &QrMatrix, size: u32, path: &Path) -> Result<()> {
    ensure_canvas_size(size)?;
    fs::write(path, render_svg(matrix, size))
        .with_context(|| format!("failed to write SVG to {}", path.display()))
}

fn ensure_canvas_size(size: u32) -> Result<()> {
    if size == 0 {
        bail!("--size must be greater than zero");
    }
    Ok(())
}

fn render_svg(matrix: &QrMatrix, size: u32) -> String {
    let mut path = String::new();
    for (x, y) in matrix.dark_modules() {
        write!(&mut path, "M{x} {y}h1v1H{x}z").expect("writing to String cannot fail");
    }

    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{size}" height="{size}" viewBox="0 0 {width} {width}" shape-rendering="crispEdges">
<rect width="100%" height="100%" fill="#fff"/>
<path d="{path}" fill="#000"/>
</svg>
"##,
        width = matrix.width
    )
}

fn render_ascii(matrix: &QrMatrix, dark: &str) -> Result<String> {
    let style = AsciiStyle::new(dark)?;
    let mut output = String::new();
    for y in 0..matrix.width {
        for x in 0..matrix.width {
            output.push_str(if matrix.is_dark(x, y) {
                &style.dark
            } else {
                &style.light
            });
        }
        output.push('\n');
    }
    Ok(output)
}

impl ErrorCorrection {
    fn ec_level(self) -> EcLevel {
        match self {
            Self::L => EcLevel::L,
            Self::M => EcLevel::M,
            Self::Q => EcLevel::Q,
            Self::H => EcLevel::H,
        }
    }
}

impl Security {
    fn requires_password(self) -> bool {
        !matches!(self, Self::NoPass)
    }

    fn qr_value(self) -> &'static str {
        match self {
            Self::Wpa => "WPA",
            Self::Wep => "WEP",
            Self::NoPass => "nopass",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_wifi_payload_matching_requested_order() {
        let payload =
            build_wifi_payload("2024shownet", Some("from-messe"), Security::Wpa, false).unwrap();
        assert_eq!(payload, "WIFI:S:2024shownet;T:WPA;P:from-messe;;");
    }

    #[test]
    fn rejects_missing_password_for_protected_networks() {
        let error = build_wifi_payload("2024shownet", None, Security::Wpa, false)
            .unwrap_err()
            .to_string();

        assert_eq!(
            error,
            "--password is required unless --security nopass is used"
        );
    }

    #[test]
    fn escapes_wifi_special_characters() {
        let escaped = escape_wifi_field(r#"a\b;c,d:e""#);
        assert_eq!(escaped, r#"a\\b\;c\,d\:e\""#);
    }

    #[test]
    fn infers_format_from_output_extension() {
        let options = output_options(Some("qr.svg"), None);

        assert!(matches!(
            options.target().unwrap(),
            OutputTarget::Svg(path) if path == Path::new("qr.svg")
        ));
    }

    #[test]
    fn defaults_to_ascii_without_output() {
        let options = output_options(None, None);

        assert!(matches!(options.target().unwrap(), OutputTarget::Ascii));
    }

    #[test]
    fn rejects_ascii_output_file() {
        let options = output_options(Some("qr.txt"), Some(OutputFormat::Ascii));
        let error = options.target().unwrap_err().to_string();

        assert_eq!(
            error,
            "terminal text output is printed to stdout; omit --output"
        );
    }

    #[test]
    fn renders_ascii_with_default_square() {
        let matrix = sample_matrix();

        assert_eq!(
            render_ascii(&matrix, DEFAULT_ASCII_CHAR).unwrap(),
            "⬜  \n  ⬜\n"
        );
    }

    #[test]
    fn renders_single_width_ascii_char_as_double_width_cell() {
        let matrix = sample_matrix();

        assert_eq!(render_ascii(&matrix, "#").unwrap(), "##  \n  ##\n");
    }

    #[test]
    fn rejects_empty_ascii_char() {
        let error = render_ascii(&sample_matrix(), "").unwrap_err().to_string();

        assert_eq!(error, "--ascii-char must not be empty");
    }

    #[test]
    fn rejects_invisible_ascii_char() {
        let error = render_ascii(&sample_matrix(), "  ")
            .unwrap_err()
            .to_string();

        assert_eq!(error, "--ascii-char must contain a visible character");
    }

    #[test]
    fn rejects_control_ascii_char() {
        let error = render_ascii(&sample_matrix(), "#\n")
            .unwrap_err()
            .to_string();

        assert_eq!(error, "--ascii-char must not contain control characters");
    }

    #[test]
    fn version_flags_split_short_and_long_output() {
        let short = Cli::try_parse_from(["wifiqr", "-V"]).unwrap_err();
        let long = Cli::try_parse_from(["wifiqr", "--version"]).unwrap_err();

        assert_eq!(short.kind(), clap::error::ErrorKind::DisplayVersion);
        assert_eq!(long.kind(), clap::error::ErrorKind::DisplayVersion);
        assert_eq!(
            short.to_string(),
            format!("wifiqr {}\n", env!("CARGO_PKG_VERSION"))
        );

        let long = long.to_string();
        assert!(long.starts_with(&format!("wifiqr {} (git ", env!("CARGO_PKG_VERSION"))));
        assert!(long.contains("; commit "));
        assert!(long.contains("; commit date "));
        assert!(long.contains("; built "));
        assert!(long.contains(") on "));
        assert_ne!(long, short.to_string());
    }

    #[test]
    fn version_flags_propagate_to_subcommands() {
        let short = Cli::try_parse_from(["wifiqr", "raw", "-V"]).unwrap_err();

        assert_eq!(short.kind(), clap::error::ErrorKind::DisplayVersion);
        assert_eq!(
            short.to_string(),
            format!("wifiqr-raw {}\n", env!("CARGO_PKG_VERSION"))
        );
    }

    fn sample_matrix() -> QrMatrix {
        QrMatrix {
            width: 2,
            modules: vec![true, false, false, true],
        }
    }

    fn output_options(output: Option<&str>, format: Option<OutputFormat>) -> OutputOptions {
        OutputOptions {
            output: output.map(PathBuf::from),
            format,
            size: 1024,
            border: 4,
            error_correction: ErrorCorrection::H,
            ascii_char: DEFAULT_ASCII_CHAR.to_string(),
        }
    }
}
