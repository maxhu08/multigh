//! Native test double so Windows exercises the same executable lookup as real `gh`.
use std::{env, fs, path::PathBuf, process, thread, time::Duration};

fn main() {
    let state = PathBuf::from(env::var_os("MGH_TEST_AUTH").unwrap());
    let selected = fs::read_to_string(state.join("selected")).unwrap();
    let args = env::args().skip(1).collect::<Vec<_>>();
    let args = args.iter().map(String::as_str).collect::<Vec<_>>();
    match args.as_slice() {
        ["--version"] => println!("gh version 2.99.0"),
        ["config", "get", "user", ..] => println!("{selected}"),
        ["auth", "status", ..] => {
            println!(
                r#"{{"hosts":{{"github.com":[{{"login":"alice","active":{},"state":"success"}},{{"login":"alice-work","active":{},"state":"success"}}]}}}}"#,
                selected == "alice",
                selected == "alice-work",
            );
        }
        ["auth", "switch", "--hostname", "github.com", "--user", target] => {
            let gate = state.join("pause-switch");
            if gate.exists() {
                fs::write(state.join("switch-started"), "").unwrap();
                for _ in 0..1500 {
                    if !gate.exists() {
                        break;
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                if gate.exists() {
                    eprintln!("test switch timed out");
                    process::exit(2);
                }
            }
            if state.join("fail-switch").exists() {
                eprintln!("test switch failed");
                process::exit(1);
            }
            fs::write(state.join("selected"), target).unwrap();
        }
        _ => panic!("Unexpected gh invocation: {args:?}"),
    }
}
