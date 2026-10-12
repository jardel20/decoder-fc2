use clap::{Parser, Subcommand, ValueEnum};
use fc2_rs::{
    analysis, decoder, downloads,
    error::{Error, Result},
    export, reader,
    records::Reference,
    tui,
    validator::{self, Profile},
    workspace,
};
use serde::Serialize;
use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

#[derive(Parser)]
#[command(
    name = "fc2",
    version,
    args_conflicts_with_subcommands = true,
    about = "FC2 inspection, active-work XLSX export and reference validation"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    /// Optional FC2 to open directly (also supports dropping a file on the executable)
    file: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Csv,
    Json,
    Xlsx,
}
#[derive(Clone, Copy, ValueEnum)]
enum WorkbookOrder {
    Leaf,
    Stored,
}

#[derive(Clone, Copy, ValueEnum)]
enum ExportMode {
    Complete,
    Simple,
}
#[derive(Clone, Copy, ValueEnum)]
enum Order {
    Source,
    Reference,
}
#[derive(Clone, Copy, ValueEnum)]
enum Mode {
    Inspect,
    Decode,
}

fn parse_offset(s: &str) -> std::result::Result<usize, String> {
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        usize::from_str_radix(hex, 16)
    } else {
        s.parse()
    }
    .map_err(|e| format!("invalid nonnegative offset/length: {e}"))
}

#[derive(Subcommand)]
enum Commands {
    /// Show the embedded application and icon licenses
    License,
    /// Interactive terminal: browse files, works, measurements and exports
    Tui {
        file: Option<PathBuf>,
        #[arg(long)]
        profile: Option<PathBuf>,
    },
    /// List works from active filesystem directories, independently of a reference
    Works {
        file: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Export active works as XLSX only or with a complete download and audit package
    Export {
        file: PathBuf,
        #[arg(short, long)]
        output: Option<PathBuf>,
        #[arg(long)]
        /// Repeat --work to select multiple works; omit to export all
        work: Vec<u16>,
        #[arg(long, value_enum, default_value = "simple")]
        mode: ExportMode,
        #[arg(long)]
        profile: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "leaf")]
        order: WorkbookOrder,
    },
    /// Serve a generated export directory on 127.0.0.1 only
    Serve {
        directory: PathBuf,
        #[arg(long, default_value_t = 8712)]
        port: u16,
    },
    /// Preserve the complete FC2, reconstructed memory and active files in a ZIP
    Extract {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Inspect any input, including unsupported or malformed files
    Inspect { file: PathBuf },
    /// Dump bytes with original file offsets (decimal or 0x hexadecimal arguments)
    Hex {
        file: PathBuf,
        #[arg(long,default_value="0",value_parser=parse_offset)]
        offset: usize,
        #[arg(long,default_value="512",value_parser=parse_offset)]
        length: usize,
    },
    /// Write structural evidence and hypotheses as JSON; does not assert scientific validity
    Analyze {
        file: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
    },
    /// Compare a documented reference selection; issue a profile only on complete agreement
    Validate {
        file: PathBuf,
        #[arg(long)]
        reference: PathBuf,
        #[arg(long)]
        reference_source: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long)]
        report: Option<PathBuf>,
    },
    /// Export only fields and records validated by a profile for this exact file hash
    Decode {
        file: PathBuf,
        #[arg(long)]
        profile: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "csv")]
        format: Format,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, value_enum, default_value = "source")]
        order: Order,
    },
    /// Process immediate FC2 files in deterministic order and isolate individual failures
    Batch {
        directory: PathBuf,
        #[arg(short, long)]
        output: PathBuf,
        #[arg(long, value_enum, default_value = "inspect")]
        mode: Mode,
        #[arg(long)]
        profile: Option<PathBuf>,
        #[arg(long, value_enum, default_value = "csv")]
        format: Format,
        #[arg(long, value_enum, default_value = "source")]
        order: Order,
    },
}

fn output_text(s: &str) -> Result<()> {
    io::stdout().lock().write_all(s.as_bytes())?;
    Ok(())
}
fn json_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut b = serde_json::to_vec_pretty(value)?;
    b.push(b'\n');
    Ok(b)
}

fn decode(file: &Path, profile: Option<&Path>, format: Format, order: Order) -> Result<Vec<u8>> {
    let path = profile.ok_or_else(|| {
        Error::Validation(
            "a --profile issued by `fc2 validate` is required; structure alone is insufficient"
                .into(),
        )
    })?;
    let p: Profile = serde_json::from_slice(&reader::read_file(path)?)?;
    let bytes = reader::read_file(file)?;
    let (report, records) = validator::decode(&bytes, &p)?;
    let doc = export::document(
        file.to_string_lossy().into_owned(),
        report,
        records,
        matches!(order, Order::Reference),
    );
    match format {
        Format::Csv => export::csv::encode(&doc),
        Format::Json => export::json::encode(&doc),
        Format::Xlsx => export::xlsx::render(&doc),
    }
}

#[derive(Serialize)]
struct BatchRow {
    source_name: String,
    source_sha256: Option<String>,
    output: Option<String>,
    status: String,
    error: Option<String>,
}
#[derive(Serialize)]
struct BatchReport {
    schema: String,
    files: Vec<BatchRow>,
    failures: usize,
}

fn run(cli: Cli) -> Result<()> {
    match cli.command.unwrap_or(Commands::Tui {
        file: cli.file,
        profile: None,
    }) {
        Commands::License => output_text(concat!(
            include_str!("../LICENSE"),
            "\nPhosphor Icons\n",
            include_str!("../licenses/Phosphor-Icons-MIT.txt")
        )),
        Commands::Tui { file, profile } => tui::run(file.as_deref(), profile.as_deref()),
        Commands::Works { file, output } => {
            let dataset = workspace::decode(&reader::read_file(&file)?)?;
            if let Some(output) = output {
                export::write_new(&output, &json_bytes(&dataset)?)
            } else {
                let mut text = format!(
                    "{} active works; {} records; {} filesystem sector CRCs verified\n",
                    dataset.works.len(),
                    dataset
                        .works
                        .iter()
                        .map(|w| w.record_count())
                        .sum::<usize>(),
                    dataset.filesystem.crc_verified_count
                );
                for work in &dataset.works {
                    text.push_str(&format!(
                        "{}\t{}\t{} records\t{} groups\t{}\n",
                        work.number,
                        work.display_name,
                        work.record_count(),
                        work.groups.len(),
                        if work.errors.is_empty() {
                            "structurally decoded"
                        } else {
                            "validation errors"
                        }
                    ));
                }
                text.push_str("External Leaf comparison remains work-specific. Missing values and unknown fields are preserved in the audit JSON.\n");
                output_text(&text)
            }
        }
        Commands::Export {
            file,
            output,
            work,
            mode,
            profile,
            order,
        } => {
            let output = output.unwrap_or_else(|| {
                let stem = file.file_stem().unwrap_or_default().to_string_lossy();
                fc2_rs::paths::new_output(
                    &fc2_rs::paths::downloads_dir(),
                    &format!("{stem}-trabalhos"),
                )
            });
            let bytes = reader::read_file(&file)?;
            let mut dataset = workspace::decode(&bytes)?;
            if let Some(profile) = profile {
                let profile: Profile = serde_json::from_slice(&reader::read_file(&profile)?)?;
                workspace::compare_reference(&bytes, &mut dataset, &profile)?;
            }
            let manifest = export::works::export_directory_options(
                &bytes,
                &dataset,
                &file.to_string_lossy(),
                &output,
                if work.is_empty() {
                    None
                } else {
                    Some(work.as_slice())
                },
                match order {
                    WorkbookOrder::Leaf => export::works::WorkbookOrder::Leaf,
                    WorkbookOrder::Stored => export::works::WorkbookOrder::Stored,
                },
                match mode {
                    ExportMode::Complete => export::works::ExportMode::Complete,
                    ExportMode::Simple => export::works::ExportMode::Simple,
                },
            )?;
            output_text(&format!(
                "Exported {} XLSX; {} records; {} groups; {} failures\nOutput: {}\n",
                manifest.successful_workbooks,
                manifest.record_count,
                manifest.group_count,
                manifest.failures,
                output.display()
            ))?;
            if manifest.failures > 0 {
                for work in &manifest.works {
                    if let Some(error) = &work.error {
                        output_text(&format!(
                            "Blocked work {} ({}): {}\n",
                            work.number, work.name, error
                        ))?;
                    }
                }
                Err(Error::Validation(format!(
                    "{} works blocked; inspect work validation errors (manifest.json in complete mode)",
                    manifest.failures
                )))
            } else {
                Ok(())
            }
        }
        Commands::Serve { directory, port } => downloads::serve(&directory, port),
        Commands::Extract { file, output } => {
            let bytes = reader::read_file(&file)?;
            let dataset = workspace::decode(&bytes)?;
            export::write_new(&output, &export::works::raw_archive(&bytes, &dataset)?)
        }
        Commands::Inspect { file } => output_text(&analysis::inspect_text(&analysis::analyze(
            &reader::read_file(&file)?,
        ))),
        Commands::Hex {
            file,
            offset,
            length,
        } => {
            if length > reader::MAX_FILE_SIZE {
                return Err(Error::Format("hex length exceeds 64 MiB limit".into()));
            }
            output_text(&reader::hexdump(
                &reader::read_file(&file)?,
                offset,
                length,
            )?)
        }
        Commands::Analyze { file, output } => export::write_new(
            &output,
            &json_bytes(&analysis::analyze(&reader::read_file(&file)?))?,
        ),
        Commands::Validate {
            file,
            reference,
            reference_source,
            output,
            report,
        } => {
            let r: Reference = serde_json::from_slice(&reader::read_file(&reference)?)?;
            if reader::hash(&reader::read_file(&reference_source)?) != r.source_reference_sha256 {
                return Err(Error::Validation(
                    "reference-source hash differs from the reference manifest".into(),
                ));
            }
            let (validation, _) = validator::validate(&reader::read_file(&file)?, &r)?;
            if let Some(path) = report {
                export::write_new(&path, &json_bytes(&validation)?)?;
            }
            output_text(&format!(
                "Matched {}/{} records; {} matching physical reconstructions; accepted={}\n",
                validation.matched_records,
                validation.expected_records,
                validation.matched_occurrences,
                validation.accepted
            ))?;
            if !validation.accepted {
                return Err(Error::Validation("reference selection has missing records or ambiguous raw variants; no profile issued".into()));
            }
            let profile = Profile {
                schema: "fc2-validation-profile-v1".into(),
                layout: decoder::LAYOUT.into(),
                tool_version: env!("CARGO_PKG_VERSION").into(),
                reference: r,
                validation,
            };
            export::write_new(&output, &json_bytes(&profile)?)
        }
        Commands::Decode {
            file,
            profile,
            format,
            output,
            order,
        } => export::write_new(&output, &decode(&file, profile.as_deref(), format, order)?),
        Commands::Batch {
            directory,
            output,
            mode,
            profile,
            format,
            order,
        } => {
            if matches!(mode, Mode::Decode) && profile.is_none() {
                return Err(Error::Validation(
                    "batch decode requires --profile; each file is checked against its hash".into(),
                ));
            }
            let mut files = std::fs::read_dir(&directory)?
                .map(|e| e.map(|entry| entry.path()))
                .collect::<std::io::Result<Vec<_>>>()?;
            files.retain(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("fc2")));
            files.sort();
            if files.is_empty() {
                return Err(Error::Format(
                    "no FC2 files in the directory (nonrecursive scan)".into(),
                ));
            }
            std::fs::create_dir_all(&output)?;
            let mut rows = Vec::new();
            let mut failures = 0;
            for file in files {
                let suffix = match mode {
                    Mode::Inspect => "analysis.json",
                    Mode::Decode => match format {
                        Format::Csv => "csv",
                        Format::Json => "json",
                        Format::Xlsx => "xlsx",
                    },
                };
                let name = file
                    .file_name()
                    .ok_or_else(|| Error::Format("missing file name".into()))?;
                let mut target = name.to_os_string();
                target.push(format!(".{suffix}"));
                let target = output.join(target);
                let source_hash = reader::read_file(&file).ok().map(|b| reader::hash(&b));
                let result = (|| {
                    let bytes = match mode {
                        Mode::Inspect => {
                            let b = reader::read_file(&file)?;
                            fc2_rs::parser::parse(&b)?;
                            json_bytes(&analysis::analyze(&b))?
                        }
                        Mode::Decode => decode(&file, profile.as_deref(), format, order)?,
                    };
                    export::write_new(&target, &bytes)
                })();
                let (status, error, destination) = match result {
                    Ok(()) => ("ok", None, Some(target.to_string_lossy().into_owned())),
                    Err(e) => {
                        failures += 1;
                        ("error", Some(e.to_string()), None)
                    }
                };
                rows.push(BatchRow {
                    source_name: file.to_string_lossy().into_owned(),
                    source_sha256: source_hash,
                    output: destination,
                    status: status.into(),
                    error,
                });
            }
            let report = BatchReport {
                schema: "fc2-batch-v1".into(),
                files: rows,
                failures,
            };
            export::write_new(&output.join("batch-report.json"), &json_bytes(&report)?)?;
            output_text(&format!(
                "Processed {} files; {} failures\n",
                report.files.len(),
                failures
            ))?;
            if failures > 0 {
                return Err(Error::Validation(
                    "batch completed with individual failures; see batch-report.json".into(),
                ));
            }
            Ok(())
        }
    }
}

fn main() -> ExitCode {
    match run(Cli::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("fc2: {e}");
            ExitCode::FAILURE
        }
    }
}
