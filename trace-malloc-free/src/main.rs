extern crate duct;
#[macro_use]
extern crate quicli;
extern crate regex;

use quicli::prelude::*;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

/// Trace the malloc and frees of some program and output the result as something that can be fed as testing/benching input to `wee_alloc`.
///
/// Depends on valgrind being installed.
#[derive(Debug, StructOpt)]
struct Cli {
    /// The file path to write the traced output into.
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output: PathBuf,

    /// The command to spawn and trace its mallocs and frees.
    command: Vec<String>,
}

main!(|cli: Cli| {
    let mut output = BufWriter::new(fs::File::create(&cli.output)?);

    let mut args = vec![
        "--trace-malloc=yes".to_string(),
        "--trace-children=yes".to_string(),
    ];
    args.extend(cli.command.iter().cloned());

    let stderr = duct::cmd("valgrind", &args)
        .stderr_capture()
        .unchecked()
        .run()?
        .stderr;
    let stderr = String::from_utf8(stderr)?;

    let malloc_re = Regex::new(
        r#"^\-\-\d+\-\- (realloc\(0x0,\d+\))?malloc\((?P<size>\d+)\) = 0x(?P<ptr>\w+)$"#,
    ).unwrap();

    let calloc_re = Regex::new(
        r#"^\-\-\d+\-\- calloc\((?P<num>\d+),(?P<size>\d+)\) = 0x(?P<ptr>\w+)$"#,
    ).unwrap();

    let realloc_re = Regex::new(
        r#"^\-\-\d+\-\- realloc\(0x(?P<orig>\w+),(?P<size>\d+)\) = 0x(?P<new>\w+)$"#,
    ).unwrap();

    // TODO: record the requested alignment and replay that as well.
    let memalign_re = Regex::new(
        r#"r#"^\-\-\d+\-\- memalign\(al \d+, size (?P<size>\d+)\) = 0x(?P<ptr>\w+)$"#,
    ).unwrap();

    let free_re = Regex::new(r#"^\-\-\d+\-\- free\(0x(?P<ptr>\w+)\)$"#).unwrap();

    let mut num_mallocs = 0;
    let mut active_mallocs = HashMap::new();

    for line in stderr.lines() {
        if let Some(captures) = malloc_re.captures(line) {
            let size: usize = captures.name("size").unwrap().as_str().parse()?;
            let ptr = usize::from_str_radix(captures.name("ptr").unwrap().as_str(), 16)?;

            active_mallocs.insert(ptr, num_mallocs);
            num_mallocs += 1;

            writeln!(&mut output, "Alloc({}),", size)?;
            continue;
        }

        if let Some(captures) = calloc_re.captures(line) {
            let num: usize = captures.name("num").unwrap().as_str().parse()?;
            let size: usize = captures.name("size").unwrap().as_str().parse()?;
            let ptr = usize::from_str_radix(captures.name("ptr").unwrap().as_str(), 16)?;

            active_mallocs.insert(ptr, num_mallocs);
            num_mallocs += 1;

            writeln!(&mut output, "Alloc({}),", num * size)?;
            continue;
        }

        if let Some(captures) = realloc_re.captures(line) {
            // Reallocs get treated as a free and new alloc.
            let orig = usize::from_str_radix(captures.name("orig").unwrap().as_str(), 16)?;
            let size: usize = captures.name("size").unwrap().as_str().parse()?;
            let new = usize::from_str_radix(captures.name("new").unwrap().as_str(), 16)?;

            if let Some(idx) = active_mallocs.remove(&orig) {
                writeln!(&mut output, "Free({}),", idx)?;
            }

            active_mallocs.insert(new, num_mallocs);
            num_mallocs += 1;

            writeln!(&mut output, "Alloc({}),", size)?;
            continue;
        }

        if let Some(captures) = memalign_re.captures(line) {
            let size: usize = captures.name("size").unwrap().as_str().parse()?;
            let ptr = usize::from_str_radix(captures.name("ptr").unwrap().as_str(), 16)?;

            active_mallocs.insert(ptr, num_mallocs);
            num_mallocs += 1;

            writeln!(&mut output, "Alloc({}),", size)?;
            continue;
        }

        if let Some(captures) = free_re.captures(line) {
            let ptr = usize::from_str_radix(captures.name("ptr").unwrap().as_str(), 16)?;
            if let Some(idx) = active_mallocs.remove(&ptr) {
                writeln!(&mut output, "Free({}),", idx)?;
            }
            continue;
        }
    }
});
