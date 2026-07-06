use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use rsomics_common::{Result, RsomicsError};

/// CHROM+POS+REF+ALT key (bcftools isec default --collapse none).
///
/// A data line with fewer than 5 columns, or a POS that is not a
/// non-negative integer, is malformed: bcftools aborts with a VCF parse
/// error rather than skipping the record, so we do the same instead of
/// silently dropping it or matching on an opaque POS string.
fn variant_key(line: &str, path: &Path, lineno: usize) -> Result<String> {
    let fields: Vec<&str> = line.splitn(6, '\t').collect();
    if fields.len() < 5 {
        return Err(RsomicsError::InvalidInput(format!(
            "{}:{lineno}: VCF parse error: expected at least 5 columns, found {}",
            path.display(),
            fields.len()
        )));
    }
    if fields[1].parse::<u64>().is_err() {
        return Err(RsomicsError::InvalidInput(format!(
            "{}:{lineno}: could not parse the position '{}'",
            path.display(),
            fields[1]
        )));
    }
    Ok(format!(
        "{}\t{}\t{}\t{}",
        fields[0], fields[1], fields[3], fields[4]
    ))
}

fn load_keys(path: &Path) -> Result<HashSet<String>> {
    let file = File::open(path)
        .map_err(|e| RsomicsError::InvalidInput(format!("{}: {e}", path.display())))?;
    let reader = BufReader::new(file);
    let mut keys = HashSet::new();
    for (i, line) in reader.lines().enumerate() {
        let line = line.map_err(RsomicsError::Io)?;
        if line.starts_with('#') {
            continue;
        }
        keys.insert(variant_key(&line, path, i + 1)?);
    }
    Ok(keys)
}

pub fn isec(a_path: &Path, b_path: &Path, output: &mut dyn Write) -> Result<u64> {
    let b_keys = load_keys(b_path)?;
    let file = File::open(a_path)
        .map_err(|e| RsomicsError::InvalidInput(format!("{}: {e}", a_path.display())))?;
    let reader = BufReader::new(file);
    let mut out = BufWriter::with_capacity(64 * 1024, output);
    let mut count: u64 = 0;

    for (i, line) in reader.lines().enumerate() {
        let line = line.map_err(RsomicsError::Io)?;
        if line.starts_with('#') {
            writeln!(out, "{line}").map_err(RsomicsError::Io)?;
            continue;
        }
        if b_keys.contains(&variant_key(&line, a_path, i + 1)?) {
            writeln!(out, "{line}").map_err(RsomicsError::Io)?;
            count += 1;
        }
    }

    out.flush().map_err(RsomicsError::Io)?;
    Ok(count)
}
