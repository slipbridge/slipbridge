use std::collections::BTreeMap;

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

    let mut by_name: BTreeMap<String, Vec<&DiscoveredPrinter>> = BTreeMap::new();
    for printer in printers {
        by_name
            .entry(printer.display_name.clone())
            .or_default()
            .push(printer);
    }

    println!(
        "Discovered {} endpoint(s) across {} printer(s):",
        printers.len(),
        by_name.len()
    );

    for (idx, (name, mut endpoints)) in by_name.into_iter().enumerate() {
        endpoints.sort_by(|a, b| a.id.cmp(&b.id));

        println!();
        println!("{}. {}", idx + 1, name);

        for endpoint in endpoints {
            println!(
                "   - {} ({}, {})",
                endpoint.id,
                endpoint.transport,
                describe_source(endpoint.source.as_deref())
            );
        }
    }

    if let Some(first) = printers.first() {
        println!();
        println!(
            "Use an endpoint id with --printer, e.g. `--printer \"{}\"`.",
            first.id
        );
    }
}

fn describe_source(source: Option<&str>) -> String {
    match source {
        Some("_pdl-datastream._tcp.local.") => "Bonjour raw socket".to_owned(),
        Some("_printer._tcp.local.") => "Bonjour line printer".to_owned(),
        Some("usb-enumeration") => "USB enumeration".to_owned(),
        Some(other) => format!("source: {other}"),
        None => "source unknown".to_owned(),
    }
}

pub fn print_human_print(result: &PrintResponse) {
    let unit = if result.output.copies == 1 {
        "copy"
    } else {
        "copies"
    };

    println!(
        "Printed {} {} to {}.",
        result.output.copies, unit, result.printer.id
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
