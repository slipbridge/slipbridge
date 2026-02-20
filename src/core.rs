use std::{fs, io::IsTerminal, time::Instant};

use crate::{
    cli::{Cli, Command, PrintersCommand},
    diagnostics,
    error::{Result, SlipbridgeError},
    escpos::{RasterOptions, build_raster_print_job},
    input::{self, InputKind},
    output::{
        DoctorResponse, InputRef, PrintOutput, PrintResponse, PrinterRef, PrintersListResponse,
        TestOutput, TestResponse, print_human_doctor, print_human_print, print_human_printers,
        print_human_test, print_json,
    },
    profiles::{PaperProfile, PrinterProfile},
    render::{RenderOptions, render_image_for_thermal},
    transport,
};

pub fn execute(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Version => {
            println!("{}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Printers(args) => execute_printers(args.command),
        Command::Doctor(args) => execute_doctor(args.json),
        Command::Print(args) => execute_print(args),
    }
}

fn execute_printers(command: PrintersCommand) -> Result<()> {
    match command {
        PrintersCommand::List(args) => {
            let printers = if args.json || !std::io::stderr().is_terminal() {
                transport::discover_printers()?
            } else {
                let mut progress = |message: &str| {
                    eprintln!("[slipbridge] {message}");
                };
                transport::discover_printers_with_progress(&mut progress)?
            };
            let response = PrintersListResponse {
                ok: true,
                command: "printers list".to_owned(),
                printers,
            };

            if args.json {
                print_json(&response)?;
            } else {
                print_human_printers(&response.printers);
            }

            Ok(())
        }
        PrintersCommand::Test(args) => {
            let discovered = transport::discover_printers().unwrap_or_default();
            let printer = transport::resolve_printer(&args.printer, &discovered)?;

            let payload = build_test_payload();
            let stats = transport::send_bytes(&printer, &payload, 1024, 2)?;

            let response = TestResponse {
                ok: true,
                command: "printers test".to_owned(),
                printer: PrinterRef {
                    id: printer.id,
                    transport: printer.transport.as_str().to_owned(),
                },
                output: TestOutput {
                    bytes_sent: stats.bytes_sent,
                    duration_ms: stats.duration_ms,
                },
            };

            if args.json {
                print_json(&response)?;
            } else {
                print_human_test(&response);
            }

            Ok(())
        }
    }
}

fn execute_doctor(json: bool) -> Result<()> {
    let report = diagnostics::run_doctor()?;
    let response = DoctorResponse {
        ok: report.is_ok(),
        command: "doctor".to_owned(),
        report,
    };

    if json {
        print_json(&response)?;
    } else {
        print_human_doctor(&response);
    }

    if response.ok {
        Ok(())
    } else {
        Err(SlipbridgeError::transport(
            "one or more doctor checks failed",
        ))
    }
}

fn execute_print(args: crate::cli::PrintArgs) -> Result<()> {
    if args.copies == 0 {
        return Err(SlipbridgeError::invalid_argument(
            "copies must be greater than 0",
        ));
    }

    let metadata = fs::metadata(&args.input).map_err(|err| {
        SlipbridgeError::invalid_argument(format!(
            "input '{}' cannot be read: {err}",
            args.input.display()
        ))
    })?;

    if !metadata.is_file() {
        return Err(SlipbridgeError::invalid_argument(format!(
            "input '{}' is not a file",
            args.input.display()
        )));
    }

    let input_kind = input::resolve_input_kind(&args.input, args.input_type)?;
    if input_kind != InputKind::Image {
        return Err(SlipbridgeError::unsupported(format!(
            "{} printing is not implemented yet; image printing is available now",
            input_kind.as_str()
        )));
    }

    let profile = PrinterProfile::for_paper(PaperProfile::from(args.paper));
    let image = input::load_oriented_image(&args.input)?;
    let rendered = render_image_for_thermal(
        image,
        RenderOptions {
            max_width: profile.printable_width_px,
        },
    )?;

    let job = build_raster_print_job(
        &rendered,
        RasterOptions {
            max_payload_bytes: profile.max_raster_payload_bytes,
            max_rows_per_command: profile.max_rows_per_command,
            cut: args.cut_enabled(),
        },
    )?;

    let discovered = transport::discover_printers().unwrap_or_default();
    let printer = transport::resolve_printer(&args.printer, &discovered)?;

    let started = Instant::now();
    let mut total_bytes = 0usize;

    for _ in 0..args.copies {
        let send_stats =
            transport::send_bytes(&printer, &job.bytes, profile.transport_chunk_bytes, 2)?;
        total_bytes += send_stats.bytes_sent;
    }
    let max_rows_per_chunk = job.rows_per_chunk.iter().copied().max().unwrap_or(0);

    let response = PrintResponse {
        ok: true,
        command: "print".to_owned(),
        printer: PrinterRef {
            id: printer.id,
            transport: printer.transport.as_str().to_owned(),
        },
        input: InputRef {
            path: args.input.to_string_lossy().to_string(),
            input_type: input_kind.as_str().to_owned(),
        },
        output: PrintOutput {
            bytes_sent: total_bytes,
            duration_ms: started.elapsed().as_millis(),
            chunks: job.chunk_count * args.copies as usize,
            max_rows_per_chunk,
            copies: args.copies,
        },
        paper: profile.paper.as_str().to_owned(),
    };

    if args.json {
        print_json(&response)?;
    } else {
        print_human_print(&response);
    }

    Ok(())
}

fn build_test_payload() -> Vec<u8> {
    let mut payload = vec![0x1B, 0x40];
    payload.extend_from_slice(b"Slipbridge printer test\n");
    payload.extend_from_slice(b"----------------------\n");
    payload.extend_from_slice(b"If you can read this, transport works.\n\n");
    payload.extend_from_slice(&[0x1B, 0x64, 0x03]);
    payload.extend_from_slice(&[0x1D, 0x56, 0x41, 0x00]);
    payload
}
