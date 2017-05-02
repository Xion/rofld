//! Le build script.

use std::env;
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use std::process::Command;
use std::str;


/// File in the $OUT_DIR where the current revision is written.
const REVISION_FILE: &'static str = "revision";


fn main() {
    // Obtain Git SHA to pass it further to the crate code via ad-hoc file.
    let mut revision_file = {
        // We cannot pass the SHA as an env!() variable to the crate code,
        // so the workaround is to write it to a file for include_str!().
        // Details: https://github.com/rust-lang/cargo/issues/2875
        let out_dir = env::var("OUT_DIR").unwrap();
        let rev_path = Path::new(&out_dir).join(REVISION_FILE);
        File::create(&rev_path).unwrap()
    };
    match git_head_sha() {
        Ok(rev) => revision_file.write_all(&rev.into_bytes()).unwrap(),
        Err(e) => println!("cargo:warning=Failed to obtain current Git SHA: {}", e),
    };
}

fn git_head_sha() -> Result<String, Box<Error>> {
    let mut cmd = Command::new("git");
    cmd.args(&["rev-parse", "--short", "HEAD"]);

    let output = try!(cmd.output());
    let sha = try!(str::from_utf8(&output.stdout[..])).trim().to_owned();
    Ok(sha)
}
