use std::{env, error::Error, fs, process, thread, time::Duration};

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().collect::<Vec<_>>();
    let mode = arguments.get(1).ok_or("missing fake worker mode")?;
    if arguments.len() < 6 {
        return Err("missing worker arguments".into());
    }
    let paths = &arguments[arguments.len() - 5..];
    let output_path = &paths[1];
    let response_path = &paths[2];
    let family_name = &paths[3];

    match mode.as_str() {
        "success" => {
            fs::write(output_path, b"wOF2fake-font")?;
            let response =
                format!("{{\"family_name\":\"{family_name}\",\"glyph_count\":2,\"advance_widths\":[500,600]}}");
            fs::write(response_path, response)?;
        }
        "invalid-asset" => fs::write(output_path, b"not-a-web-font")?,
        "missing-asset" => fs::write(response_path, b"{}")?,
        "malformed-response" => {
            fs::write(output_path, b"wOF2fake-font")?;
            fs::write(response_path, b"not-json")?;
        }
        "process-failure" => process::exit(7),
        "timeout" => thread::sleep(Duration::from_secs(1)),
        _ => return Err(format!("unknown fake worker mode: {mode}").into()),
    }

    Ok(())
}
