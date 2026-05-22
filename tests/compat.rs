use std::path::PathBuf;
use std::process::{Command, Stdio};
fn ours() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_rsomics-vcf-isec"))
}
fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name)
}

fn have(tool: &str) -> bool {
    Command::new(tool)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

/// Sorted CHROM,POS,REF,ALT keys of data records.
fn keys(vcf: &[u8]) -> Vec<String> {
    let mut v: Vec<String> = String::from_utf8_lossy(vcf)
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let c: Vec<&str> = l.split('\t').collect();
            (c.len() >= 5).then(|| format!("{}:{}:{}:{}", c[0], c[1], c[3], c[4]))
        })
        .collect();
    v.sort();
    v
}

#[test]
fn runs_with_two_files() {
    let out = Command::new(ours())
        .args(["-a"])
        .arg(fixture("a.vcf"))
        .args(["-b"])
        .arg(fixture("b.vcf"))
        .output()
        .expect("spawn");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

// Shared variants (A∩B, reported from A) must match `bcftools isec -n=2 -w1`.
#[test]
fn shared_matches_bcftools_isec() {
    if !have("bcftools") || !have("bgzip") || !have("tabix") {
        eprintln!("skipping: bcftools/bgzip/tabix not found");
        return;
    }
    let dir = std::env::temp_dir().join("rsomics-vcf-isec-compat");
    let _ = std::fs::create_dir_all(&dir);
    let prep = |name: &str| -> PathBuf {
        let plain = dir.join(name);
        std::fs::copy(fixture(name), &plain).unwrap();
        let gz = dir.join(format!("{name}.gz"));
        let g = std::fs::File::create(&gz).unwrap();
        assert!(
            Command::new("bgzip")
                .arg("-c")
                .arg(&plain)
                .stdout(g)
                .status()
                .unwrap()
                .success()
        );
        assert!(
            Command::new("tabix")
                .args(["-fp", "vcf"])
                .arg(&gz)
                .status()
                .unwrap()
                .success()
        );
        gz
    };
    let a_gz = prep("a.vcf");
    let b_gz = prep("b.vcf");

    let ours_out = Command::new(ours())
        .arg("-a")
        .arg(fixture("a.vcf"))
        .arg("-b")
        .arg(fixture("b.vcf"))
        .output()
        .unwrap();
    let bcf_out = Command::new("bcftools")
        .args(["isec", "-n=2", "-w1"])
        .arg(&a_gz)
        .arg(&b_gz)
        .output()
        .unwrap();
    assert!(bcf_out.status.success());
    assert_eq!(keys(&ours_out.stdout), keys(&bcf_out.stdout));
}
