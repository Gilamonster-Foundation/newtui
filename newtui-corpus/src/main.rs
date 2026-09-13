use std::io::{Read as _, Write as _};

use newtui_corpus::{check, fixtures, model::Artifact};

fn run() -> Result<(), String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let output = match args.as_slice() {
        [command] if command == "export-dial" => fixtures::dial()?.to_json()?,
        [command, path] if matches!(command.as_str(), "check" | "canonical") => {
            let file = std::fs::File::open(path).map_err(|error| error.to_string())?;
            let mut bytes = Vec::new();
            file.take(newtui_corpus::model::MAX_ARTIFACT_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())?;
            let artifact = Artifact::from_json(&bytes)?;
            if command == "canonical" {
                artifact.canonical_bytes()?
            } else {
                if artifact.corpus.component != fixtures::DIAL {
                    return Err(
                        "this example consumer only implements newtui.example-dial/v1".into(),
                    );
                }
                let claims = fixtures::claims();
                let claims: Vec<&dyn newtui::Property> =
                    claims.iter().map(|p| p.as_ref()).collect();
                let observations = check(
                    &artifact,
                    fixtures::Dial::from_seed,
                    &claims,
                    fixtures::Dial::intent,
                )?;
                format!(
                    "conformant: {observations} observations, {} transitions\n",
                    artifact.corpus.exploration.transitions
                )
                .into_bytes()
            }
        }
        _ => return Err("usage: newtui-corpus export-dial | check FILE | canonical FILE".into()),
    };
    std::io::stdout()
        .write_all(&output)
        .map_err(|error| error.to_string())
}

fn main() -> std::process::ExitCode {
    match run() {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("newtui-corpus: {error}");
            std::process::ExitCode::FAILURE
        }
    }
}
