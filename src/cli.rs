use crate::cli_error::{CliError, file_error};
use crate::cli_output::{OutputFile, check_target, write_outputs};
use clap::Parser;
use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use svg2vector::{Options, Size};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Convert SVG icons to Android VectorDrawable XML without Java or an Android SDK"
)]
pub(crate) struct Cli {
    #[arg(help = "SVG file, directory of direct .svg children, or '-' for stdin")]
    input: PathBuf,
    #[arg(
        short,
        long,
        help = "Output .xml file, or required output directory for directory input"
    )]
    output: Option<PathBuf>,
    #[arg(long, value_name = "DP", value_parser = positive_number, help = "Override intrinsic Android width")]
    width: Option<f64>,
    #[arg(long, value_name = "DP", value_parser = positive_number, help = "Override intrinsic Android height")]
    height: Option<f64>,
    #[arg(long, value_name = "WIDTHxHEIGHT", value_parser = canvas_size, help = "Center artwork on a new viewport without stretching")]
    canvas: Option<Size>,
    #[arg(long, help = "Replace existing output files")]
    force: bool,
}

impl Cli {
    pub(crate) fn run(&self) -> Result<(), CliError> {
        let options = Options {
            width: self.width,
            height: self.height,
            canvas: self.canvas,
        };
        if self.input == Path::new("-") {
            let mut source = String::new();
            io::stdin().lock().read_to_string(&mut source)?;
            return self.convert_single(&source, &options);
        }
        let metadata = fs::metadata(&self.input).map_err(|error| file_error(&self.input, error))?;
        if metadata.is_dir() {
            return self.convert_directory(&options);
        }
        if !metadata.is_file() {
            return Err(CliError::Usage(
                "input must be a regular file, a directory, or '-'",
            ));
        }
        let source =
            fs::read_to_string(&self.input).map_err(|error| file_error(&self.input, error))?;
        self.convert_single(&source, &options)
    }

    fn convert_single(&self, source: &str, options: &Options) -> Result<(), CliError> {
        let xml = svg2vector::convert(source, options).map_err(|source| CliError::Conversion {
            path: self.input.clone(),
            source,
        })?;
        match &self.output {
            Some(path) => {
                check_target(path, self.force)?;
                if self.input != Path::new("-") && path.exists() {
                    let input = fs::canonicalize(&self.input)
                        .map_err(|error| file_error(&self.input, error))?;
                    let output = fs::canonicalize(path).map_err(|error| file_error(path, error))?;
                    if input == output {
                        return Err(CliError::OverwriteInput(path.clone()));
                    }
                }
                write_outputs(
                    vec![OutputFile {
                        path: path.clone(),
                        xml,
                    }],
                    self.force,
                )
            }
            None => {
                let mut stdout = io::stdout().lock();
                stdout.write_all(xml.as_bytes())?;
                stdout.flush()?;
                Ok(())
            }
        }
    }

    fn convert_directory(&self, options: &Options) -> Result<(), CliError> {
        let target = self.output.as_ref().ok_or(CliError::Usage(
            "directory input requires --output DIRECTORY",
        ))?;
        if target.exists() && !target.is_dir() {
            return Err(CliError::Usage(
                "directory input requires an output directory",
            ));
        }
        let entries = fs::read_dir(&self.input).map_err(|error| file_error(&self.input, error))?;
        let mut inputs = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|error| file_error(&self.input, error))?;
            let path = entry.path();
            if path.extension().is_some_and(|extension| extension == "svg")
                && entry
                    .file_type()
                    .map_err(|error| file_error(&path, error))?
                    .is_file()
            {
                inputs.push(path);
            }
        }
        inputs.sort();
        if inputs.is_empty() {
            return Err(CliError::EmptyDirectory(self.input.clone()));
        }
        let mut outputs = Vec::with_capacity(inputs.len());
        for input in inputs {
            let name = input
                .file_name()
                .ok_or_else(|| CliError::InvalidName(input.clone()))?;
            let path = target.join(name).with_extension("xml");
            check_target(&path, self.force)?;
            let source = fs::read_to_string(&input).map_err(|error| file_error(&input, error))?;
            let xml =
                svg2vector::convert(&source, options).map_err(|source| CliError::Conversion {
                    path: input,
                    source,
                })?;
            outputs.push(OutputFile { path, xml });
        }
        write_outputs(outputs, self.force)
    }
}

fn positive_number(value: &str) -> Result<f64, String> {
    let value: f64 = value
        .parse()
        .map_err(|_| "expected a positive finite number".to_owned())?;
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err("expected a positive finite number".to_owned())
    }
}

fn canvas_size(value: &str) -> Result<Size, String> {
    let (width, height) = value
        .split_once('x')
        .ok_or_else(|| "expected WIDTHxHEIGHT, such as 24x24".to_owned())?;
    Size::new(positive_number(width)?, positive_number(height)?).map_err(|error| error.to_string())
}
