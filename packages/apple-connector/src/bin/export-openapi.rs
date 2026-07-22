use std::{env, error::Error, fs, path::PathBuf, process};

use apple_connector::build_openapi_spec;

fn main() -> Result<(), Box<dyn Error>> {
    let output = env::args().nth(1).map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("usage: export-openapi <output-path>");
        process::exit(1);
    });

    let spec = build_openapi_spec();
    let json = spec.to_pretty_json()?;
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(output, json)?;
    Ok(())
}
