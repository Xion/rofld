//! Le build script.

extern crate rustc_version;


use std::env;
use std::error::Error;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::str;


fn main() {
    match git_head_sha() {
        Ok(rev) => pass_metadata("REVISION", &rev).unwrap(),
        Err(e) => println!("cargo:warning=Failed to obtain current Git SHA: {}", e),
    };
    match compiler_signature() {
        Ok(sig) => pass_metadata("COMPILER", &sig).unwrap(),
        Err(e) => println!("cargo:warning=Failed to obtain compiler information: {}", e),
    };
}

fn pass_metadata<P: AsRef<Path>>(kind: P, data: &str) -> io::Result<()> {
    // We cannot data as an env!() variable to the crate code,
    // so the workaround is to write it to a file for include_str!().
    // Details: https://github.com/rust-lang/cargo/issues/2875
    let out_dir = env::var("OUT_DIR").unwrap();
    let path = Path::new(&out_dir).join(kind);
    let mut file = File::create(&path)?;
    file.write_all(data.as_bytes())
}


fn git_head_sha() -> Result<String, Box<Error>> {
    let mut cmd = Command::new("git");
    cmd.args(&["rev-parse", "--short", "HEAD"]);

    let output = try!(cmd.output());
    let sha = try!(str::from_utf8(&output.stdout[..])).trim().to_owned();
    Ok(sha)
}

fn compiler_signature() -> Result<String, Box<Error>> {
    let rustc = rustc_version::version_meta()?;
    let signature = format!("{channel} {version} on {host}",
        version = rustc.short_version_string,
        channel = format!("{:?}", rustc.channel).to_lowercase(),
        host = rustc.host);
    Ok(signature)
}
