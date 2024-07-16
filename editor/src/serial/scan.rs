use std::{
    collections::HashMap,
    fs::read_dir,
    path::{Path, PathBuf},
    process::Command,
    str,
};

use eyre::Context;

/// Scan for the keyboard serial device
pub fn scan_for_serial() -> eyre::Result<Option<PathBuf>> {
    log::info!("scanning for keyboard serial device");

    let mut syspaths = vec![];

    // Reqursively scan all "/sys/bus/usb/devices/usb*" folders for files called "dev"
    // and get the paths to the parent folders of those "dev" files.
    for f in read_dir("/sys/bus/usb/devices/")? {
        let f = f?;
        let file_name = f.file_name();
        let Some(name) = file_name.to_str() else {
            continue;
        };

        if !name.starts_with("usb") {
            continue;
        }

        let mut paths = scan_usb_for_devs(&f.path())?;
        syspaths.append(&mut paths);
    }

    log::debug!("checking these devices: {syspaths:#?}");

    for syspath in syspaths {
        let syspath = syspath.to_string_lossy();
        let devname = Command::new("udevadm")
            .args(["info", "-q", "name", "-p", &syspath])
            .output()
            .wrap_err("failed to run udevadmn to query dev name")?;

        let devname = str::from_utf8(&devname.stdout)
            .wrap_err("failed to parse udevadm output as utf-8")?
            .trim();

        let devname = Path::new(devname);

        // Ignore USB hubs and such
        if devname.starts_with("bus/") {
            continue;
        }

        let devpath = Path::new("/dev").join(devname);

        let properties = Command::new("udevadm")
            .args(["info", "-q", "property", "--export", "-p", &syspath])
            .output()
            .wrap_err("failed to run udevadmn to query properities")?;

        let properties = str::from_utf8(&properties.stdout)
            .wrap_err("failed to parse udevadm output as utf-8")?;

        let properties = parse_env(properties);
        let Some(properties) = parse_properties(&properties) else {
            continue;
        };

        log::debug!("{devpath:?}: {properties:#?}");

        let is_serial_device = [
            properties.model == "Tangentbord1",
            properties.vendor == "Tux",
            properties.vendor_id == "b00b",
            properties.usb_type == "generic",
            properties.usb_driver == "cdc_acm",
        ]
        .into_iter()
        .all(|b| b);

        if is_serial_device {
            return Ok(Some(devpath));
        }
    }

    Ok(None)
}

#[allow(dead_code)]
#[derive(Debug)]
struct Properties<'a> {
    model: &'a str,
    serial: &'a str,
    serial_short: &'a str,
    vendor: &'a str,
    vendor_id: &'a str,
    usb_type: &'a str,
    usb_driver: &'a str,
}

fn parse_properties<'a>(properties: &HashMap<&'a str, &'a str>) -> Option<Properties<'a>> {
    Some(Properties {
        model: properties.get("ID_MODEL")?,
        serial: properties.get("ID_SERIAL")?,
        serial_short: properties.get("ID_SERIAL_SHORT")?,
        vendor: properties.get("ID_VENDOR")?,
        vendor_id: properties.get("ID_VENDOR_ID")?,
        usb_type: properties.get("ID_USB_TYPE")?,
        usb_driver: properties.get("ID_USB_DRIVER")?,
    })
}

fn parse_env(s: &str) -> HashMap<&str, &str> {
    s.lines()
        .filter_map(|line| {
            let (key, val) = line.split_once('=')?;
            let val = val.trim_matches('\'');
            Some((key, val))
        })
        .collect()
}

fn scan_usb_for_devs(p: &Path) -> eyre::Result<Vec<PathBuf>> {
    let mut out = vec![];

    for f in read_dir(p)? {
        let f = f?;
        let meta = f.metadata()?;
        let path = f.path();
        if meta.is_dir() {
            let mut results = scan_usb_for_devs(&path)?;
            out.append(&mut results);
        } else if meta.is_file() && f.file_name() == "dev" {
            if let Some(parent) = path.parent() {
                out.push(parent.to_owned());
            }
        }
    }

    Ok(out)
}
