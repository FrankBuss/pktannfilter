use std::io::{self, BufRead, BufReader, Read, Write};
use std::process::{Child, Command, Stdio};
use std::thread::{spawn, JoinHandle};
use std::{env, process};

fn filter(line: &String, filters: &Vec<&str>, output: &mut dyn Write) {
    // test if any of the filter strings is in the current line, then ignore it
    if filters.iter().any(|filter| line.contains(filter)) {
        // let line = "filtered: ".to_string() + &line.clone();
        // output.write_all(line.as_bytes()).expect("couldn't write");
        return;
    }

    // print the line
    output.write_all(line.as_bytes()).expect("couldn't write");
}

fn filter_channel(
    mut input: Box<dyn Read + Send>,
    mut output: Box<dyn Write + Send>,
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
                filter(&line, &filters, &mut output);
                buf.clear();
            }
        }
    })
}

fn filter_channels(child: &mut Child) {
    // create 2 threads to filter stdout and stderr
    let stdout_input = Box::new(child.stdout.take().expect("error getting stdout"));
    let stdout_output = Box::new(io::stdout());
    let stdout_thread = filter_channel(stdout_input, stdout_output);
    let stderr_input = Box::new(child.stderr.take().expect("error getting stderr"));
    let stderr_output = Box::new(io::stderr());
    let stderr_thread = filter_channel(stderr_input, stderr_output);

    // wait until process end
    child.wait().expect("wait error");

    // wait until theads end
    stdout_thread.join().expect("error joining stdout thread");
    stderr_thread.join().expect("error joining stderr thread");
}

fn main() {
    // get command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("usage: {} packetcrypt_rs arguments", args[0]);
        process::exit(1);
    }

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
        Ok(mut child) => filter_channels(&mut child),
        Err(_) => eprintln!("can't start miner program {}", args[1]),
    }
}
