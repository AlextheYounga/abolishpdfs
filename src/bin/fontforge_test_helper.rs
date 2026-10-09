use std::{env, error::Error, fs, process, thread, time::Duration};

use abolishpdfs::fonts::FontJobSpec;

fn main() -> Result<(), Box<dyn Error>> {
    let arguments = env::args().collect::<Vec<_>>();
    let mode = arguments.get(1).ok_or("missing fake worker mode")?;
    if arguments.len() < 6 {
        return Err("missing worker arguments".into());
    }
    let paths = &arguments[arguments.len() - 4..];
    let input_path = &paths[0];
    let output_path = &paths[1];
    let response_path = &paths[2];
    let request_path = &paths[3];
    let spec: FontJobSpec = serde_json::from_str(&fs::read_to_string(request_path)?)?;
    let _ = input_path;

    match mode.as_str() {
        "success" => {
            fs::write(output_path, b"wOF2fake-font")?;
            let widths = spec
                .records
                .iter()
                .enumerate()
                .map(|(index, _)| u16::try_from(600 + index * 10))
                .collect::<Result<Vec<_>, _>>()?;
            let response = serde_json::json!({
                "family_name": spec.family_name,
                "glyph_count": spec.records.len() + 1,
                "units_per_em": 1000,
                "advance_widths": widths,
            });
            fs::write(response_path, serde_json::to_vec(&response)?)?;
        }
        "invalid-asset" => fs::write(output_path, b"not-a-web-font")?,
        "missing-asset" => fs::write(response_path, b"{}")?,
        "malformed-response" => {
            fs::write(output_path, b"wOF2fake-font")?;
            fs::write(response_path, b"not-json")?;
        }
        "invalid-widths" => {
            fs::write(output_path, b"wOF2fake-font")?;
            fs::write(
                response_path,
                format!(
                    "{{\"family_name\":\"{}\",\"glyph_count\":{},\"units_per_em\":1000,\"advance_widths\":[]}}",
                    spec.family_name,
                    spec.records.len() + 1
                ),
            )?;
        }
        "invalid-units" => {
            fs::write(output_path, b"wOF2fake-font")?;
            fs::write(
                response_path,
                format!(
                    "{{\"family_name\":\"{}\",\"glyph_count\":{},\"units_per_em\":0,\"advance_widths\":[]}}",
                    spec.family_name,
                    spec.records.len() + 1
                ),
            )?;
        }
        "process-failure" => process::exit(7),
        "timeout" => thread::sleep(Duration::from_secs(1)),
        _ => return Err(format!("unknown fake worker mode: {mode}").into()),
    }

    Ok(())
}
