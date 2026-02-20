use serde::Serialize;

use crate::{
    diagnostics::{CheckStatus, DoctorReport},
    error::Result,
    transport::DiscoveredPrinter,
};

#[derive(Clone, Debug, Serialize)]
pub struct PrinterRef {
    pub id: String,
    pub transport: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct InputRef {
    pub path: String,
    #[serde(rename = "type")]
    pub input_type: String,
}

#[derive(Copy, Clone, Debug, Serialize)]
pub struct PrintOutput {
    pub bytes_sent: usize,
    pub duration_ms: u128,
    pub chunks: usize,
    pub max_rows_per_chunk: u16,
    pub copies: u16,
}

#[derive(Clone, Debug, Serialize)]
pub struct PrintResponse {
    pub ok: bool,
    pub command: String,
    pub printer: PrinterRef,
    pub input: InputRef,
    pub output: PrintOutput,
    pub paper: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct PrintersListResponse {
    pub ok: bool,
    pub command: String,
    pub printers: Vec<DiscoveredPrinter>,
}

#[derive(Clone, Debug, Serialize)]
pub struct TestResponse {
    pub ok: bool,
    pub command: String,
    pub printer: PrinterRef,
    pub output: TestOutput,
}

#[derive(Copy, Clone, Debug, Serialize)]
pub struct TestOutput {
    pub bytes_sent: usize,
    pub duration_ms: u128,
}

#[derive(Clone, Debug, Serialize)]
pub struct DoctorResponse {
    pub ok: bool,
    pub command: String,
    pub report: DoctorReport,
}

pub fn print_json<T: Serialize>(value: &T) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub fn print_human_printers(printers: &[DiscoveredPrinter]) {
    if printers.is_empty() {
        println!("No printers discovered.");
        return;
    }

    for printer in printers {
        println!(
            "{}\t{}\t{}\t{}",
            printer.id, printer.display_name, printer.transport, printer.address
        );
    }
}

pub fn print_human_print(result: &PrintResponse) {
    println!(
        "Printed {} copy/copies to {}.",
        result.output.copies, result.printer.id
    );
    println!(
        "Bytes sent: {} ({} ms, {} raster chunks, max {} rows/chunk)",
        result.output.bytes_sent,
        result.output.duration_ms,
        result.output.chunks,
        result.output.max_rows_per_chunk
    );
}

pub fn print_human_test(result: &TestResponse) {
    println!(
        "Printer test sent to {} ({} bytes, {} ms).",
        result.printer.id, result.output.bytes_sent, result.output.duration_ms
    );
}

pub fn print_human_doctor(result: &DoctorResponse) {
    for check in &result.report.checks {
        let status = match check.status {
            CheckStatus::Pass => "PASS",
            CheckStatus::Warn => "WARN",
            CheckStatus::Fail => "FAIL",
        };

        println!("[{status}] {}: {}", check.name, check.detail);
    }

    if result.ok {
        println!("Doctor completed without failing checks.");
    } else {
        println!("Doctor found one or more failing checks.");
    }
}
