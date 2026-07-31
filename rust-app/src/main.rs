mod convert;
mod dxf_read;
mod dxf_write;
mod geom;
mod gui;
mod svg_read;

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        if let Err(e) = gui::run() {
            eprintln!("GUI error: {e}");
            return ExitCode::FAILURE;
        }
        return ExitCode::SUCCESS;
    }

    // CLI: dxf_trutops_cleaner_rs file.dxf [-t 0.1] [-o out.dxf]
    let mut tolerance = 0.1_f64;
    let mut output: Option<PathBuf> = None;
    let mut inputs: Vec<PathBuf> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-t" | "--tolerance" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Brak wartosci dla -t");
                    return ExitCode::FAILURE;
                }
                tolerance = args[i].parse().unwrap_or(0.1);
            }
            "-o" | "--output" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("Brak sciezki dla -o");
                    return ExitCode::FAILURE;
                }
                output = Some(PathBuf::from(&args[i]));
            }
            s if s.starts_with('-') => {
                eprintln!("Nieznana opcja: {s}");
                return ExitCode::FAILURE;
            }
            s => inputs.push(PathBuf::from(s)),
        }
        i += 1;
    }

    if inputs.is_empty() {
        eprintln!("Podaj plik DXF/SVG lub uruchom bez argumentow (GUI).");
        return ExitCode::FAILURE;
    }
    if output.is_some() && inputs.len() > 1 {
        eprintln!("-o tylko dla jednego pliku.");
        return ExitCode::FAILURE;
    }

    let mut failures = 0;
    for (idx, input) in inputs.iter().enumerate() {
        println!("Wczytywanie: {}", input.display());
        let out = if idx == 0 {
            output.clone()
        } else {
            None
        };
        match convert::convert_file(input, tolerance, out) {
            Ok(r) => {
                println!(
                    "Wierzcholki: {} -> {} (tolerancja {})",
                    r.before, r.after, tolerance
                );
                println!("Kontury: {}", r.contours);
                if r.splines > 0 {
                    println!(
                        "{}: {} krzywych/sciazek -> lukow: {}",
                        r.source, r.splines, r.arc_segments
                    );
                }
                println!("Zrodlo: {}", r.source);
                println!("Zapisano DXF: {}", r.output.display());
            }
            Err(e) => {
                eprintln!("Blad: {e}");
                failures += 1;
            }
        }
    }
    if failures > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
