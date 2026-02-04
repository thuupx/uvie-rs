use std::io::{self, Read};
use uvie::{InputMethod, UltraFastViEngine};

fn main() {
    let mut engine = UltraFastViEngine::new();

    let mut args = std::env::args().skip(1);
    let mut method = InputMethod::Telex;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                println!(
                    "Usage: uvie [--mode telex|vni]\n\n  --mode telex|vni   Select input method (default: telex)"
                );
                return;
            }
            "--mode" => {
                let Some(v) = args.next() else {
                    eprintln!("--mode requires a value: telex|vni");
                    return;
                };
                method = match v.as_str() {
                    "telex" => InputMethod::Telex,
                    "vni" => InputMethod::Vni,
                    _ => {
                        eprintln!("Unsupported mode: {v} (use telex|vni)");
                        return;
                    }
                };
            }
            _ => {
                eprintln!("Unknown argument: {arg} (use --help)");
                return;
            }
        }
    }

    engine.set_input_method(method);
    let mut stdin = io::stdin().lock();

    let mut buf = [0u8; 1];

    loop {
        if let Ok(n) = stdin.read(&mut buf) {
            if n == 0 {
                continue;
            }
            let b = buf[0];

            if b == b'\n' {
                let out = engine.feed(' ');
                println!("\n{}", out);
                continue;
            }

            if b == 3 {
                break;
            }

            let c = b as char;
            let out = engine.feed(c);
            print!("\r{}", out);
            io::Write::flush(&mut io::stdout()).unwrap();
        }
    }
}
