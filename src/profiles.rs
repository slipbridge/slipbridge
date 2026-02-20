use crate::cli::PaperArg;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum PaperProfile {
    Mm58,
    Mm80,
}

impl From<PaperArg> for PaperProfile {
    fn from(value: PaperArg) -> Self {
        match value {
            PaperArg::Mm58 => PaperProfile::Mm58,
            PaperArg::Mm80 => PaperProfile::Mm80,
        }
    }
}

impl PaperProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            PaperProfile::Mm58 => "58mm",
            PaperProfile::Mm80 => "80mm",
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct PrinterProfile {
    pub paper: PaperProfile,
    pub printable_width_px: u32,
    pub max_raster_payload_bytes: usize,
    pub transport_chunk_bytes: usize,
    pub max_rows_per_command: u16,
}

impl PrinterProfile {
    pub fn for_paper(paper: PaperProfile) -> Self {
        match paper {
            // Common 58mm ESC/POS printers: ~384 dots printable width at 8 dots/mm.
            PaperProfile::Mm58 => Self {
                paper,
                printable_width_px: 384,
                max_raster_payload_bytes: 3_072,
                transport_chunk_bytes: 4_096,
                max_rows_per_command: 255,
            },
            // Common 80mm ESC/POS printers: ~576 dots printable width at 8 dots/mm.
            PaperProfile::Mm80 => Self {
                paper,
                printable_width_px: 576,
                max_raster_payload_bytes: 4_096,
                transport_chunk_bytes: 4_096,
                max_rows_per_command: 255,
            },
        }
    }
}
