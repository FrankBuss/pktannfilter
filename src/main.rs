use const_format::concatcp;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::thread::{spawn, JoinHandle};
use std::{env, process};
#[cfg(target_os="windows")]
use ansi_term;

// standard ANSI escape codes
pub const ANSI_RESET: &str = "\x1b[0m";
pub const ANSI_BRIGHT: &str = "\x1b[1m";
pub const ANSI_DIM: &str = "\x1b[2m";
pub const ANSI_UNDERSCORE: &str = "\x1b[4m";
pub const ANSI_BLINK: &str = "\x1b[5m";
pub const ANSI_REVERSE: &str = "\x1b[7m";
pub const ANSI_HIDDEN: &str = "\x1b[8m";

pub const ANSI_FG_BLACK: &str = "\x1b[30m";
pub const ANSI_FG_RED: &str = "\x1b[31m";
pub const ANSI_FG_GREEN: &str = "\x1b[32m";
pub const ANSI_FG_YELLOW: &str = "\x1b[33m";
pub const ANSI_FG_BLUE: &str = "\x1b[34m";
pub const ANSI_FG_MAGENTA: &str = "\x1b[35m";
pub const ANSI_FG_CYAN: &str = "\x1b[36m";
pub const ANSI_FG_WHITE: &str = "\x1b[37m";

pub const ANSI_BG_BLACK: &str = "\x1b[40m";
pub const ANSI_BG_RED: &str = "\x1b[41m";
pub const ANSI_BG_GREEN: &str = "\x1b[42m";
pub const ANSI_BG_YELLOW: &str = "\x1b[43m";
pub const ANSI_BG_BLUE: &str = "\x1b[44m";
pub const ANSI_BG_MAGENTA: &str = "\x1b[45m";
pub const ANSI_BG_CYAN: &str = "\x1b[46m";
pub const ANSI_BG_WHITE: &str = "\x1b[47m";

// used in the Go packetwallet program
pub const ANSI_COLOR_DBG: &str = concatcp!(ANSI_DIM, ANSI_FG_WHITE);
pub const ANSI_COLOR_WARN: &str = concatcp!(ANSI_BRIGHT, ANSI_FG_YELLOW);
pub const ANSI_COLOR_ERR: &str = concatcp!(ANSI_BRIGHT, ANSI_FG_RED);
pub const ANSI_COLOR_CRIT: &str = concatcp!(ANSI_BRIGHT, ANSI_FG_BLACK, ANSI_BG_RED);

// color definitions for goodrate
pub const GOODRATE_COLOR_POOL: &str = concatcp!(ANSI_FG_BLACK, ANSI_BG_CYAN);
pub const GOODRATE_COLOR_PERCENT_0_50: &str = ANSI_FG_RED;
pub const GOODRATE_COLOR_PERCENT_50_75: &str = concatcp!(ANSI_BRIGHT, ANSI_FG_YELLOW);
pub const GOODRATE_COLOR_PERCENT_75_100: &str = ANSI_FG_GREEN;

fn filter(line: &String, filters: &Vec<&str>, output: &mut dyn Write, pools: &Vec<String>) {
    // test if any of the filter strings is in the current line, then ignore it
    if filters.iter().any(|filter| line.contains(filter)) {
        // let line = "filtered: ".to_string() + &line.clone();
        // output.write_all(line.as_bytes()).unwrap();
        return;
    }

    // test if it is a goodrate line
    let gr = "goodrate: [";
    if let Some(start) = line.find(gr) {
        let list_and_end = line[start + gr.len()..].to_string();
        if let Some(end) = list_and_end.find("]") {
            let before_list = line[..start + gr.len()].to_string();
            let list = list_and_end[..end].to_string();
            let after_list = list_and_end[end..].to_string();
            let list = list.split(", ").collect::<Vec<&str>>();

            output.write_all(before_list.as_bytes()).unwrap();
            for (i, percent) in list.iter().enumerate() {
                if i > 0 {
                    output.write_all(", ".as_bytes()).unwrap();
                }
                output.write_all(GOODRATE_COLOR_POOL.as_bytes()).unwrap();
                output.write_all(pools[i].as_bytes()).unwrap();
                output.write_all(ANSI_RESET.as_bytes()).unwrap();
                output.write_all(": ".as_bytes()).unwrap();
                let num = percent
                    .chars()
                    .take_while(|c| c.is_numeric())
                    .collect::<String>()
                    .parse::<usize>()
                    .unwrap();
                if num < 50 {
                    output
                        .write_all(GOODRATE_COLOR_PERCENT_0_50.as_bytes())
                        .unwrap();
                } else if num < 75 {
                    output
                        .write_all(GOODRATE_COLOR_PERCENT_50_75.as_bytes())
                        .unwrap();
                } else {
                    output
                        .write_all(GOODRATE_COLOR_PERCENT_75_100.as_bytes())
                        .unwrap();
                }
                output.write_all(percent.as_bytes()).unwrap();
                output.write_all(ANSI_RESET.as_bytes()).unwrap();
            }
            output.write_all(after_list.as_bytes()).unwrap();

            return;
        }
    }

    // print the line
    output.write_all(line.as_bytes()).unwrap();
}

fn filter_channel(
    mut input: Box<dyn Read + Send>,
    mut output: Box<dyn Write + Send>,
    pools: Arc<Vec<String>>,
) -> JoinHandle<()> {
    spawn(move || {
        // define filters which are filtered out
        let filters = vec!["Error uploading ann batch", "Failed to make request to"];

        loop {
            // create a BufReader for the input stream
            let mut reader = BufReader::new(&mut input);

            // end thread, if it is empty (EOF)
            let buf_test = reader.fill_buf().unwrap();
            if buf_test.len() == 0 {
                break;
            }

            // read it line by line and filter it
            let mut buf = vec![];
            while let Ok(_) = reader.read_until(b'\n', &mut buf) {
                if buf.is_empty() {
                    break;
                }
                let line = String::from_utf8_lossy(&buf).to_string();
                filter(&line, &filters, &mut output, &pools);
                buf.clear();
            }
        }
    })
}

fn filter_channels(child: &mut Child, pools: Arc<Vec<String>>) {
    // create 2 threads to filter stdout and stderr
    let stdout_input = Box::new(child.stdout.take().expect("error getting stdout"));
    let stdout_output = Box::new(io::stdout());
    let stdout_thread = filter_channel(stdout_input, stdout_output, pools.clone());
    let stderr_input = Box::new(child.stderr.take().expect("error getting stderr"));
    let stderr_output = Box::new(io::stderr());
    let stderr_thread = filter_channel(stderr_input, stderr_output, pools.clone());

    // wait until process end
    child.wait().expect("wait error");

    // wait until theads end
    stdout_thread.join().expect("error joining stdout thread");
    stderr_thread.join().expect("error joining stderr thread");
}

fn main() {
    #[cfg(target_os="windows")]
    ansi_term::enable_ansi_support().unwrap();

    // get command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: {} packetcrypt_rs arguments", args[0]);
        process::exit(1);
    }

    // get all pool names
    let mut pools = Vec::new();
    for arg in args.clone() {
        if arg.starts_with("http://") {
            pools.push(arg[7..].to_string());
        }
    }
    let pools = Arc::new(pools);

    // create miner command
    let mut command = Command::new(env::args().skip(1).next().unwrap());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    // add all miner command line arguments
    for arg in env::args().skip(2) {
        command.arg(arg);
    }

    // start miner
    match command.spawn() {
        Ok(mut child) => filter_channels(&mut child, pools),
        Err(_) => eprintln!("can't start miner program {}", args[1]),
    }
}
