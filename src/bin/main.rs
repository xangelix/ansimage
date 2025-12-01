use std::path::PathBuf;

use ansimage::{
    Advanced, Characters, Colors, Dithering, Settings, Size, UnicodeCharSet, convert, palettes,
    settings::CharacterMode,
};
use clap::Parser;

/// A simple command-line tool to convert images into terminal art.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Path to the input image file.
    #[arg(required = true)]
    input: PathBuf,

    /// Optional path to write the output text file.
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// If set, does not print the output to the terminal.
    #[arg(long)]
    quiet: bool,

    /// Output width in characters.
    #[arg(long)]
    width: Option<usize>,

    /// Output height in characters.
    #[arg(long)]
    height: Option<usize>,

    /// Uncompressed output (no ANSI color code compression).
    #[arg(short, long)]
    uncompressed: bool,
}

fn main() -> color_eyre::Result<()> {
    let cli = Cli::parse();

    let settings = Settings {
        size: Size {
            width: cli.width.unwrap_or(80),
            height: cli.height.unwrap_or(80),
            ..Default::default()
        },
        characters: Characters {
            mode: CharacterMode::Unicode(UnicodeCharSet::Quarter),
            ..Default::default()
        },
        colors: Colors {
            palette: palettes::COLOR_PALETTE_SWEETIE16.to_vec(),
            is_truecolor: false,
        },
        advanced: Advanced {
            dithering: Dithering {
                is_enabled: false,
                ..Default::default()
            },
            compression: !cli.uncompressed,
            ..Default::default()
        },
    };

    // The `convert` function handles opening and decoding the image.
    let output_str = convert(&cli.input, &settings)?;

    if !cli.quiet {
        println!("{output_str}");
    }

    if let Some(output_path) = cli.output {
        std::fs::write(output_path, output_str)?;
    }

    Ok(())
}
