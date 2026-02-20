use std::{
    collections::{HashMap, HashSet},
    io::Write,
    net::{SocketAddr, TcpStream, ToSocketAddrs},
    thread,
    time::{Duration, Instant},
};

use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent};
use rusb::{Context, Device, DeviceDescriptor, Direction, TransferType, UsbContext};
use serde::Serialize;

use crate::error::{Result, SlipbridgeError};

const EPSON_VENDOR_ID: u16 = 0x04B8;
const USB_TIMEOUT: Duration = Duration::from_secs(3);
const NETWORK_TIMEOUT: Duration = Duration::from_secs(3);
const MDNS_BROWSE_TIMEOUT: Duration = Duration::from_millis(1_500);
const PROBE_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Clone, Debug, Serialize)]
pub struct DiscoveredPrinter {
    pub id: String,
    pub display_name: String,
    pub transport: String,
    pub address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TransportKind {
    Usb,
    Tcp,
}

impl TransportKind {
    pub fn as_str(self) -> &'static str {
        match self {
            TransportKind::Usb => "usb",
            TransportKind::Tcp => "tcp",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ResolvedPrinter {
    pub id: String,
    pub transport: TransportKind,
    pub target: PrinterTarget,
}

#[derive(Clone, Debug)]
pub enum PrinterTarget {
    Usb(UsbTarget),
    Tcp(TcpTarget),
}

#[derive(Clone, Debug)]
pub struct UsbTarget {
    pub vendor_id: u16,
    pub product_id: u16,
    pub bus_number: Option<u8>,
    pub device_address: Option<u8>,
}

#[derive(Clone, Debug)]
pub struct TcpTarget {
    pub host: String,
    pub port: u16,
}

#[derive(Copy, Clone, Debug, Serialize)]
pub struct SendStats {
    pub bytes_sent: usize,
    pub duration_ms: u128,
}

pub fn discover_printers() -> Result<Vec<DiscoveredPrinter>> {
    let mut sink = |_message: &str| {};
    discover_printers_with_progress(&mut sink)
}

pub fn discover_printers_with_progress(
    on_progress: &mut dyn FnMut(&str),
) -> Result<Vec<DiscoveredPrinter>> {
    let mut printers = Vec::new();

    on_progress("Scanning USB printers");
    if let Ok(mut usb_printers) = discover_usb_printers() {
        printers.append(&mut usb_printers);
    }

    on_progress("Scanning network printers via Bonjour (mDNS) and TCP probes");
    printers.extend(discover_network_printers(on_progress));
    on_progress("Finalizing printer list");

    let mut seen = HashSet::new();
    printers.retain(|printer| seen.insert(printer.id.clone()));
    printers.sort_by(|a, b| a.id.cmp(&b.id));

    Ok(printers)
}

pub fn resolve_printer(
    selector: &str,
    discovered: &[DiscoveredPrinter],
) -> Result<ResolvedPrinter> {
    if let Some(spec) = selector.strip_prefix("usb://") {
        let target = parse_usb_target(spec)?;
        return Ok(ResolvedPrinter {
            id: format!("usb://{:04x}:{:04x}", target.vendor_id, target.product_id),
            transport: TransportKind::Usb,
            target: PrinterTarget::Usb(target),
        });
    }

    if let Some(spec) = selector.strip_prefix("tcp://") {
        let target = parse_tcp_target(spec)?;
        return Ok(ResolvedPrinter {
            id: format!("tcp://{}:{}", target.host, target.port),
            transport: TransportKind::Tcp,
            target: PrinterTarget::Tcp(target),
        });
    }

    if let Ok(target) = parse_tcp_target(selector) {
        return Ok(ResolvedPrinter {
            id: format!("tcp://{}:{}", target.host, target.port),
            transport: TransportKind::Tcp,
            target: PrinterTarget::Tcp(target),
        });
    }

    let normalized_selector = selector.to_ascii_lowercase();

    let mut exact_matches = discovered
        .iter()
        .filter(|printer| printer.id.eq_ignore_ascii_case(selector))
        .cloned()
        .collect::<Vec<_>>();

    if exact_matches.is_empty() {
        exact_matches = discovered
            .iter()
            .filter(|printer| {
                printer
                    .display_name
                    .to_ascii_lowercase()
                    .contains(&normalized_selector)
            })
            .cloned()
            .collect::<Vec<_>>();
    }

    match exact_matches.len() {
        0 => Err(SlipbridgeError::not_found(format!(
            "no printer matched selector '{}'",
            selector
        ))),
        1 => resolve_discovered_printer(&exact_matches.remove(0)),
        _ => Err(SlipbridgeError::invalid_argument(format!(
            "printer selector '{}' is ambiguous; use a full id",
            selector
        ))),
    }
}

pub fn send_bytes(
    printer: &ResolvedPrinter,
    bytes: &[u8],
    chunk_size: usize,
    retries: u8,
) -> Result<SendStats> {
    let started = Instant::now();

    match &printer.target {
        PrinterTarget::Tcp(target) => send_tcp_bytes(target, bytes, chunk_size, retries)?,
        PrinterTarget::Usb(target) => send_usb_bytes(target, bytes, chunk_size, retries)?,
    }

    Ok(SendStats {
        bytes_sent: bytes.len(),
        duration_ms: started.elapsed().as_millis(),
    })
}

fn discover_usb_printers() -> Result<Vec<DiscoveredPrinter>> {
    let context = Context::new()?;
    let devices = context.devices()?;

    let mut printers = Vec::new();

    for device in devices.iter() {
        let descriptor = match device.device_descriptor() {
            Ok(descriptor) => descriptor,
            Err(_) => continue,
        };

        if !is_likely_receipt_printer(&device, &descriptor) {
            continue;
        }

        let (manufacturer, product) = read_strings(&device, &descriptor);
        let bus_number = device.bus_number();
        let device_address = device.address();

        let fallback_name = format!(
            "USB {:04x}:{:04x}",
            descriptor.vendor_id(),
            descriptor.product_id()
        );

        let display_name = match (manufacturer, product) {
            (Some(m), Some(p)) => format!("{m} {p}"),
            (Some(m), None) => format!("{m} {fallback_name}"),
            (None, Some(p)) => p,
            (None, None) => fallback_name,
        };

        printers.push(DiscoveredPrinter {
            id: format!(
                "usb://{:04x}:{:04x}:{}:{}",
                descriptor.vendor_id(),
                descriptor.product_id(),
                bus_number,
                device_address
            ),
            display_name,
            transport: "usb".to_owned(),
            address: format!("bus {bus_number} device {device_address}"),
            source: Some("usb-enumeration".to_owned()),
        });
    }

    Ok(printers)
}

fn discover_network_printers(on_progress: &mut dyn FnMut(&str)) -> Vec<DiscoveredPrinter> {
    let mdns = match ServiceDaemon::new() {
        Ok(mdns) => mdns,
        Err(_) => {
            on_progress("Bonjour (mDNS) unavailable; skipping network browse");
            return Vec::new();
        }
    };

    let mut by_id = HashMap::<String, DiscoveredPrinter>::new();
    let mut probe_9100_cache = HashMap::<String, bool>::new();

    for (service_type, description) in [
        ("_pdl-datastream._tcp.local.", "raw socket printers"),
        ("_printer._tcp.local.", "line printer services"),
    ] {
        let message = format!("Browsing Bonjour: {description} ({service_type})");
        on_progress(&message);

        for resolved in browse_mdns_services(&mdns, service_type, MDNS_BROWSE_TIMEOUT) {
            for discovered in discovered_from_mdns_service(&resolved, &mut probe_9100_cache) {
                by_id.entry(discovered.id.clone()).or_insert(discovered);
            }
        }

        let _ = mdns.stop_browse(service_type);
    }

    let _ = mdns.shutdown();

    by_id.into_values().collect()
}

fn browse_mdns_services(
    mdns: &ServiceDaemon,
    service_type: &str,
    timeout: Duration,
) -> Vec<ResolvedService> {
    let receiver = match mdns.browse(service_type) {
        Ok(receiver) => receiver,
        Err(_) => return Vec::new(),
    };

    let deadline = Instant::now() + timeout;
    let mut resolved = HashMap::new();

    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }

        let remaining = deadline.saturating_duration_since(now);

        match receiver.recv_timeout(remaining) {
            Ok(ServiceEvent::ServiceResolved(info)) => {
                resolved.insert(info.get_fullname().to_owned(), *info);
            }
            Ok(_) => {}
            Err(_) => break,
        }
    }

    resolved.into_values().collect()
}

fn discovered_from_mdns_service(
    service: &ResolvedService,
    probe_9100_cache: &mut HashMap<String, bool>,
) -> Vec<DiscoveredPrinter> {
    let host = service.get_hostname().trim_end_matches('.');
    if host.is_empty() {
        return Vec::new();
    }

    let service_port = service.get_port();
    let display_name = service
        .get_property_val_str("ty")
        .map(str::to_owned)
        .unwrap_or_else(|| service_instance_name(service.get_fullname()));

    let source = Some(service.ty_domain.clone());

    let mut candidate_ports = HashSet::new();

    if service_port > 0 {
        candidate_ports.insert(service_port);
    }

    if service_port != 9100 {
        let cache_key = host.to_ascii_lowercase();
        let has_9100 = *probe_9100_cache
            .entry(cache_key)
            .or_insert_with(|| can_connect_service(service, 9100, PROBE_TIMEOUT));

        if has_9100 {
            candidate_ports.insert(9100);
        }
    }

    candidate_ports
        .into_iter()
        .map(|port| DiscoveredPrinter {
            id: format!("tcp://{host}:{port}"),
            display_name: display_name.clone(),
            transport: "tcp".to_owned(),
            address: format!("{host}:{port}"),
            source: source.clone(),
        })
        .collect()
}

fn service_instance_name(fullname: &str) -> String {
    if let Some((instance, _rest)) = fullname.split_once("._") {
        return instance.to_owned();
    }

    fullname.trim_end_matches('.').to_owned()
}

fn can_connect_service(service: &ResolvedService, port: u16, timeout: Duration) -> bool {
    for address in service.get_addresses() {
        let socket_addr = SocketAddr::new(address.to_ip_addr(), port);
        if TcpStream::connect_timeout(&socket_addr, timeout).is_ok() {
            return true;
        }
    }

    let host = service.get_hostname().trim_end_matches('.');
    if host.is_empty() {
        return false;
    }

    can_connect_host(host, port, timeout)
}

fn can_connect_host(host: &str, port: u16, timeout: Duration) -> bool {
    let addrs = match (host, port).to_socket_addrs() {
        Ok(addrs) => addrs,
        Err(_) => return false,
    };

    for addr in addrs {
        if TcpStream::connect_timeout(&addr, timeout).is_ok() {
            return true;
        }
    }

    false
}

fn resolve_discovered_printer(printer: &DiscoveredPrinter) -> Result<ResolvedPrinter> {
    if let Some(spec) = printer.id.strip_prefix("tcp://") {
        let target = parse_tcp_target(spec)?;
        return Ok(ResolvedPrinter {
            id: format!("tcp://{}:{}", target.host, target.port),
            transport: TransportKind::Tcp,
            target: PrinterTarget::Tcp(target),
        });
    }

    if let Some(spec) = printer.id.strip_prefix("usb://") {
        let target = parse_usb_target(spec)?;
        return Ok(ResolvedPrinter {
            id: format!("usb://{:04x}:{:04x}", target.vendor_id, target.product_id),
            transport: TransportKind::Usb,
            target: PrinterTarget::Usb(target),
        });
    }

    Err(SlipbridgeError::invalid_argument(format!(
        "unsupported discovered printer id '{}'",
        printer.id
    )))
}

fn send_tcp_bytes(target: &TcpTarget, bytes: &[u8], chunk_size: usize, retries: u8) -> Result<()> {
    let mut stream = connect_tcp_with_timeout(&target.host, target.port, NETWORK_TIMEOUT)?;
    stream.set_write_timeout(Some(NETWORK_TIMEOUT))?;

    let chunk_size = chunk_size.max(1);

    for chunk in bytes.chunks(chunk_size) {
        let mut offset = 0usize;
        let mut attempts = 0u8;

        while offset < chunk.len() {
            match stream.write(&chunk[offset..]) {
                Ok(0) => return Err(SlipbridgeError::transport("network write returned 0 bytes")),
                Ok(written) => {
                    offset += written;
                    attempts = 0;
                }
                Err(err) => {
                    if attempts >= retries {
                        return Err(SlipbridgeError::transport(format!(
                            "network write failed: {err}"
                        )));
                    }

                    attempts += 1;
                    thread::sleep(Duration::from_millis(30));
                }
            }
        }
    }

    stream.flush()?;
    Ok(())
}

fn send_usb_bytes(target: &UsbTarget, bytes: &[u8], chunk_size: usize, retries: u8) -> Result<()> {
    let session = open_usb_session(target)?;
    let chunk_size = chunk_size.max(1);

    for chunk in bytes.chunks(chunk_size) {
        let mut offset = 0usize;
        let mut attempts = 0u8;

        while offset < chunk.len() {
            match session
                .handle
                .write_bulk(session.out_endpoint, &chunk[offset..], USB_TIMEOUT)
            {
                Ok(0) => return Err(SlipbridgeError::transport("USB write returned 0 bytes")),
                Ok(written) => {
                    offset += written;
                    attempts = 0;
                }
                Err(err) => {
                    if attempts >= retries {
                        return Err(SlipbridgeError::transport(format!(
                            "USB write failed: {err}"
                        )));
                    }

                    attempts += 1;
                    thread::sleep(Duration::from_millis(30));
                }
            }
        }
    }

    Ok(())
}

struct UsbSession {
    handle: rusb::DeviceHandle<Context>,
    interface_number: u8,
    out_endpoint: u8,
}

impl Drop for UsbSession {
    fn drop(&mut self) {
        let _ = self.handle.release_interface(self.interface_number);
    }
}

fn open_usb_session(target: &UsbTarget) -> Result<UsbSession> {
    let context = Context::new()?;
    let devices = context.devices()?;

    for device in devices.iter() {
        let descriptor = match device.device_descriptor() {
            Ok(descriptor) => descriptor,
            Err(_) => continue,
        };

        if descriptor.vendor_id() != target.vendor_id
            || descriptor.product_id() != target.product_id
        {
            continue;
        }

        if let Some(bus_number) = target.bus_number
            && device.bus_number() != bus_number
        {
            continue;
        }

        if let Some(device_address) = target.device_address
            && device.address() != device_address
        {
            continue;
        }

        let Some((interface_number, out_endpoint)) = find_bulk_out_endpoint(&device) else {
            continue;
        };

        let handle = device.open()?;
        let _ = handle.set_auto_detach_kernel_driver(true);

        #[cfg(not(target_os = "windows"))]
        {
            if let Ok(true) = handle.kernel_driver_active(interface_number) {
                let _ = handle.detach_kernel_driver(interface_number);
            }
        }

        handle.claim_interface(interface_number)?;

        return Ok(UsbSession {
            handle,
            interface_number,
            out_endpoint,
        });
    }

    Err(SlipbridgeError::not_found(format!(
        "USB printer {:04x}:{:04x} not found",
        target.vendor_id, target.product_id
    )))
}

fn find_bulk_out_endpoint<T: UsbContext>(device: &Device<T>) -> Option<(u8, u8)> {
    let config_descriptor = device
        .active_config_descriptor()
        .ok()
        .or_else(|| device.config_descriptor(0).ok())?;

    for interface in config_descriptor.interfaces() {
        for descriptor in interface.descriptors() {
            for endpoint in descriptor.endpoint_descriptors() {
                if endpoint.transfer_type() == TransferType::Bulk
                    && endpoint.direction() == Direction::Out
                {
                    return Some((descriptor.interface_number(), endpoint.address()));
                }
            }
        }
    }

    None
}

fn parse_usb_target(value: &str) -> Result<UsbTarget> {
    let fields = value.split(':').collect::<Vec<_>>();

    if fields.len() != 2 && fields.len() != 4 {
        return Err(SlipbridgeError::invalid_argument(
            "USB selector must be VID:PID or VID:PID:BUS:ADDRESS",
        ));
    }

    let vendor_id = parse_u16_hex(fields[0], "vendor id")?;
    let product_id = parse_u16_hex(fields[1], "product id")?;

    let bus_number = if fields.len() == 4 {
        Some(parse_u8_auto(fields[2], "bus number")?)
    } else {
        None
    };

    let device_address = if fields.len() == 4 {
        Some(parse_u8_auto(fields[3], "device address")?)
    } else {
        None
    };

    Ok(UsbTarget {
        vendor_id,
        product_id,
        bus_number,
        device_address,
    })
}

fn parse_tcp_target(value: &str) -> Result<TcpTarget> {
    let (host, port) = value
        .rsplit_once(':')
        .ok_or_else(|| SlipbridgeError::invalid_argument("tcp selector must be host:port"))?;

    if host.is_empty() {
        return Err(SlipbridgeError::invalid_argument(
            "tcp selector host cannot be empty",
        ));
    }

    let port: u16 = port.parse().map_err(|_| {
        SlipbridgeError::invalid_argument(format!("invalid tcp port '{}': expected 1-65535", port))
    })?;

    if port == 0 {
        return Err(SlipbridgeError::invalid_argument(
            "tcp port must be greater than 0",
        ));
    }

    Ok(TcpTarget {
        host: host.to_owned(),
        port,
    })
}

fn parse_u16_hex(value: &str, field: &str) -> Result<u16> {
    let normalized = value.trim().trim_start_matches("0x");

    u16::from_str_radix(normalized, 16).map_err(|_| {
        SlipbridgeError::invalid_argument(format!("invalid {field} '{normalized}', expected hex"))
    })
}

fn parse_u8_auto(value: &str, field: &str) -> Result<u8> {
    let normalized = value.trim();

    if let Some(hex) = normalized.strip_prefix("0x") {
        return u8::from_str_radix(hex, 16).map_err(|_| {
            SlipbridgeError::invalid_argument(format!("invalid {field} '{normalized}'"))
        });
    }

    normalized
        .parse::<u8>()
        .map_err(|_| SlipbridgeError::invalid_argument(format!("invalid {field} '{normalized}'")))
}

fn is_likely_receipt_printer<T: UsbContext>(
    device: &Device<T>,
    descriptor: &DeviceDescriptor,
) -> bool {
    if descriptor.vendor_id() == EPSON_VENDOR_ID {
        return true;
    }

    if descriptor.class_code() == 0x07 {
        return true;
    }

    let config_descriptor = match device
        .active_config_descriptor()
        .ok()
        .or_else(|| device.config_descriptor(0).ok())
    {
        Some(config_descriptor) => config_descriptor,
        None => return false,
    };

    config_descriptor.interfaces().any(|interface| {
        interface
            .descriptors()
            .any(|descriptor| descriptor.class_code() == 0x07)
    })
}

fn read_strings<T: UsbContext>(
    device: &Device<T>,
    descriptor: &DeviceDescriptor,
) -> (Option<String>, Option<String>) {
    let handle = match device.open() {
        Ok(handle) => handle,
        Err(_) => return (None, None),
    };

    let manufacturer = handle.read_manufacturer_string_ascii(descriptor).ok();
    let product = handle.read_product_string_ascii(descriptor).ok();

    (manufacturer, product)
}

fn connect_tcp_with_timeout(host: &str, port: u16, timeout: Duration) -> Result<TcpStream> {
    let addrs = (host, port).to_socket_addrs().map_err(|err| {
        SlipbridgeError::transport(format!("failed to resolve {host}:{port}: {err}"))
    })?;

    let mut last_error = None;

    for addr in addrs {
        match TcpStream::connect_timeout(&addr, timeout) {
            Ok(stream) => return Ok(stream),
            Err(err) => last_error = Some(err),
        }
    }

    let detail = match last_error {
        Some(err) => format!(": {err}"),
        None => String::from(": no socket addresses resolved"),
    };

    Err(SlipbridgeError::transport(format!(
        "failed to connect to {host}:{port} within {} ms{detail}",
        timeout.as_millis()
    )))
}

#[cfg(test)]
mod tests {
    use super::{parse_tcp_target, parse_usb_target};

    #[test]
    fn parses_tcp_target_host_port() {
        let target = parse_tcp_target("printer.local:9100").expect("target should parse");
        assert_eq!(target.host, "printer.local");
        assert_eq!(target.port, 9100);
    }

    #[test]
    fn rejects_tcp_target_with_zero_port() {
        let err = parse_tcp_target("printer.local:0").expect_err("port 0 should fail");
        assert!(err.to_string().contains("greater than 0"));
    }

    #[test]
    fn parses_usb_target_with_bus_and_address() {
        let target = parse_usb_target("04b8:0e15:1:2").expect("target should parse");
        assert_eq!(target.vendor_id, 0x04b8);
        assert_eq!(target.product_id, 0x0e15);
        assert_eq!(target.bus_number, Some(1));
        assert_eq!(target.device_address, Some(2));
    }
}
