//! The `.orusapp` format: a zip of a built bundle (`index.html`, `waid-package.json`
//! and the rest), with the files at the zip's top level.

use std::io::{Cursor, Read, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};
use zip::write::SimpleFileOptions;

/// Most files an archive may hold, and the most it may unpack to. They stop a small
/// download from filling the disk.
const MAX_ENTRIES: usize = 20_000;
const MAX_UNPACKED_BYTES: u64 = 1 << 30;

/// Zip the files under `bundle`.
pub fn pack(bundle: &Path) -> Result<Vec<u8>> {
    let mut out = Cursor::new(Vec::new());
    let mut zip = zip::ZipWriter::new(&mut out);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    add_dir(&mut zip, bundle, bundle, options)?;
    zip.finish()?;
    Ok(out.into_inner())
}

fn add_dir<W: Write + std::io::Seek>(
    zip: &mut zip::ZipWriter<W>,
    root: &Path,
    dir: &Path,
    options: SimpleFileOptions,
) -> Result<()> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?.collect::<Result<_, _>>()?;
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let path = entry.path();
        let kind = entry.file_type()?;
        let name = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        if kind.is_dir() {
            add_dir(zip, root, &path, options)?;
        } else if kind.is_file() {
            zip.start_file(name, options)?;
            zip.write_all(&std::fs::read(&path)?)?;
        } else {
            bail!("{}: symlinks are not packed", path.display());
        }
    }
    Ok(())
}

/// Unpack `bytes` into the empty folder `dest`. A name that would leave `dest`
/// (`..`, an absolute path) or a link fails the whole unpack.
pub fn unpack(bytes: &[u8], dest: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).context("not a .orusapp (zip) file")?;
    if zip.len() > MAX_ENTRIES {
        bail!("the archive holds more than {MAX_ENTRIES} files");
    }
    let mut total = 0_u64;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let Some(relative) = file.enclosed_name() else {
            bail!("the archive has an unsafe path: {}", file.name());
        };
        if file.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000) {
            bail!("the archive has a link: {}", file.name());
        }
        let target = dest.join(relative);
        if file.is_dir() {
            std::fs::create_dir_all(&target)?;
            continue;
        }
        total += file.size();
        if total > MAX_UNPACKED_BYTES {
            bail!("the archive unpacks to more than {MAX_UNPACKED_BYTES} bytes");
        }
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut data = Vec::new();
        file.by_ref()
            .take(MAX_UNPACKED_BYTES + 1)
            .read_to_end(&mut data)?;
        std::fs::write(&target, data)?;
    }
    Ok(())
}
