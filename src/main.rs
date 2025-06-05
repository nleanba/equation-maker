use std::{
    ffi::OsStr,
    fs::{read_to_string, write},
    path::PathBuf,
    process::{Command, Stdio},
};

use clap::Parser;
use miette::{Context, IntoDiagnostic};
use regex::Regex;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(value_name = "FILE")]
    source: PathBuf,
    #[arg(short, long, value_name = "SVG DIR")]
    output: PathBuf,
    /// path to use for img src for SVG DIR
    #[arg(short, long, value_name = "PATH")]
    path: String,
}

fn main() -> miette::Result<()> {
    let cli = Cli::parse();
    let html = read_to_string(&cli.source)
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot read file {}", &cli.source.display()))?;
    let new = create_equations(
        &html,
        &cli.source
            .file_stem()
            .wrap_err("provided source file does not have file_stem")?,
        &cli.output,
        &cli.path,
    )?;
    write(&cli.source, new)
        .into_diagnostic()
        .wrap_err_with(|| format!("cannot write file {}", &cli.source.display()))?;
    Ok(())
}

const STYLE: &str = r#">
<style>
    :root {
        color-scheme: light dark;
    }
    .typst-text use {
        fill: light-dark(oklch(0.35 0.035 215), oklch(0.98 0.015 215)) !important;
    }
</style>"#;

fn create_equations(
    haystack: &str,
    name: &OsStr,
    output: &PathBuf,
    path: &str,
) -> miette::Result<String> {
    let re = Regex::new(r"\[\[\[(.*?)\]\]\]").expect("should always be able to create regex");
    let mut new = String::with_capacity(haystack.len());
    let mut last_match = 0;
    for (i, captures) in re.captures_iter(haystack).enumerate() {
        let m = captures.get(0).unwrap();
        let (equation, display) = captures[1]
            .strip_prefix('!')
            .map_or((&captures[1], false), |s| (s, true));
        new.push_str(&haystack[last_match..m.start()]);

        let args = [
            "--input",
            &format!("d={}", display),
            "--input",
            &format!(r#"eq={}"#, equation),
        ];

        println!("{args:?}");

        let svg = Command::new("typst")
            .args(["compile", "./template.typ", "-", "--format", "svg"])
            .args(args)
            .stderr(Stdio::inherit())
            .output()
            .into_diagnostic()
            .wrap_err("failed to spawn typst compile")?;

        if !svg.status.success() {
            return Err(miette::Report::msg("typst failed to create svg"));
        }

        let svg = str::from_utf8(&svg.stdout)
            .into_diagnostic()
            .wrap_err_with(|| format!("Invalid UTF-8 sequence in svg"))?
            .replacen('>', STYLE, 1);

        let file_name = format!("{}-{}.svg", name.display(), i);
        write(output.join(&file_name), svg)
            .into_diagnostic()
            .wrap_err("Could not write svg")?;

        let baseline = Command::new("typst")
            .args([
                "query",
                "./template.typ",
                "<down>",
                "--field",
                "value",
                "--one",
            ])
            .args(args)
            .stderr(Stdio::inherit())
            .output()
            .into_diagnostic()
            .wrap_err("failed to spawn typst query")?;

        if !baseline.status.success() {
            return Err(miette::Report::msg("typst failed to calculate baseline"));
        }

        let baseline: f64 = str::from_utf8(&baseline.stdout)
            .into_diagnostic()
            .wrap_err_with(|| format!("Invalid UTF-8 sequence in baseline"))?
            .trim()
            .parse()
            .into_diagnostic()
            .wrap_err_with(|| format!("failed to parse baseline"))?;
        let baseline = -baseline;

        let equation = equation.replace('"', r#"\""#);

        let class = if display {
            "equation block"
        } else {
            "equation"
        };

        new.push_str(&format!(r#"<img alt="{equation}" src="{path}{file_name}" style="vertical-align: {baseline}mm;" class="{class}">"#));
        last_match = m.end();
    }
    new.push_str(&haystack[last_match..]);
    Ok(new)
}
