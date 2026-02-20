//! Printer discovery and ESC/POS communication.

/// A connection to a thermal receipt printer.
pub struct Printer {
    // TODO: transport handle (USB, serial, network)
}

impl Printer {
    /// Discover available printers on the system.
    pub fn discover() -> Result<Vec<Printer>, Box<dyn std::error::Error>> {
        todo!("printer discovery")
    }

    /// Print raw ESC/POS bytes.
    pub fn print_raw(&self, _data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        todo!("raw printing")
    }
}
