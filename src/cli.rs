use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "slipbridge",
    version,
    about = "Local-first thermal printer CLI"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Print an input file to a thermal printer.
    Print(PrintArgs),
    /// List or test available printers.
    Printers(PrintersArgs),
    /// Run local diagnostics.
    Doctor(DoctorArgs),
    /// Show the version.
    Version,
}

#[derive(Debug, Args)]
pub struct PrintArgs {
    /// Path to the input file.
    pub input: PathBuf,

    /// Printer selector: discovered id/name, usb://VID:PID, tcp://host:port, or host:port.
    #[arg(long)]
    pub printer: String,

    /// Paper width profile.
    #[arg(long, value_enum, default_value_t = PaperArg::Mm80)]
    pub paper: PaperArg,

    /// Force input type.
    #[arg(long = "type", value_enum)]
    pub input_type: Option<InputTypeArg>,

    /// Explicitly request cutting at the end of the print (default behavior).
    #[arg(long, action = ArgAction::SetTrue, conflicts_with = "no_cut")]
    pub cut: bool,

    /// Disable cutting at the end of the print.
    #[arg(long = "no-cut", action = ArgAction::SetTrue, conflicts_with = "cut")]
    pub no_cut: bool,

    /// Number of copies.
    #[arg(long, default_value_t = 1)]
    pub copies: u16,

    /// Emit machine-readable JSON output.
    #[arg(long, action = ArgAction::SetTrue)]
    pub json: bool,
}

impl PrintArgs {
    pub fn cut_enabled(&self) -> bool {
        if self.no_cut {
            return false;
        }

        if self.cut {
            return true;
        }

        true
    }
}

#[derive(Debug, Args)]
pub struct PrintersArgs {
    #[command(subcommand)]
    pub command: PrintersCommand,
}

#[derive(Debug, Subcommand)]
pub enum PrintersCommand {
    /// List discoverable printers.
    List(ListPrintersArgs),
    /// Send a small connectivity test payload.
    Test(TestPrinterArgs),
}

#[derive(Debug, Args)]
pub struct ListPrintersArgs {
    /// Emit machine-readable JSON output.
    #[arg(long, action = ArgAction::SetTrue)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct TestPrinterArgs {
    /// Printer selector: discovered id/name, usb://VID:PID, tcp://host:port, or host:port.
    #[arg(long)]
    pub printer: String,

    /// Emit machine-readable JSON output.
    #[arg(long, action = ArgAction::SetTrue)]
    pub json: bool,
}

#[derive(Debug, Args)]
pub struct DoctorArgs {
    /// Emit machine-readable JSON output.
    #[arg(long, action = ArgAction::SetTrue)]
    pub json: bool,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum PaperArg {
    #[value(name = "58mm")]
    Mm58,
    #[value(name = "80mm")]
    Mm80,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum InputTypeArg {
    Image,
    Markdown,
    Text,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{PaperArg, PrintArgs};

    fn base_print_args() -> PrintArgs {
        PrintArgs {
            input: PathBuf::from("sample.txt"),
            printer: "tcp://printer.local:9100".to_owned(),
            paper: PaperArg::Mm80,
            input_type: None,
            cut: false,
            no_cut: false,
            copies: 1,
            json: false,
        }
    }

    #[test]
    fn cut_is_enabled_by_default() {
        let args = base_print_args();
        assert!(args.cut_enabled());
    }

    #[test]
    fn no_cut_overrides_default_cut_behavior() {
        let mut args = base_print_args();
        args.no_cut = true;
        assert!(!args.cut_enabled());
    }
}
