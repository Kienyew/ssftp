mod client;
use client::SSFTPClient;
use ssftp::StatusCode;
use ssftp::utils::socket_addr_validator;

use std::error::Error;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter};
use std::net::SocketAddr;
use std::path::Path;

// cli arguments
use clap::{App, Arg, ArgMatches};


/// Perform a get request
fn run_get(client: SSFTPClient, matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let remote_path = Path::new(matches.value_of("remote-path").unwrap());
    let local_path = Path::new(matches.value_of("local-path").unwrap());
    let mut response = client.get(Path::new(remote_path))?;

    if let StatusCode::OK = response.status_code {
        let content_length = response
            .headers
            .get("content-length")
            .unwrap()
            .as_i64()
            .unwrap();
        let mut output_stream = BufWriter::new(File::create(local_path)?);
        std::io::copy(response.payload_stream.as_mut(), &mut output_stream)?;
        println!(
            "The file with {} bytes successfully downloaded to {}",
            content_length,
            local_path.to_str().unwrap(),
        );
    } else {
        println!(
            "Response status code is not OK: {}",
            response.status_code.to_string()
        );
    }

    Ok(())
}

/// Perform a ir request
fn run_dir(client: SSFTPClient, matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let remote_path = Path::new(matches.value_of("remote-path").unwrap());
    let response = client.dir(remote_path)?;

    if let StatusCode::OK = response.status_code {
        let reader = BufReader::new(response.payload_stream);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("{}", line)
            }
        }
    } else {
        println!(
            "Response status code is not OK: {}",
            response.status_code.to_string()
        );
    }

    Ok(())
}

/// Perform an info request
fn run_info(client: SSFTPClient, matches: &ArgMatches) -> Result<(), Box<dyn Error>> {
    let remote_path = Path::new(matches.value_of("remote-path").unwrap());
    let response = client.info(remote_path)?;

    if let StatusCode::OK = response.status_code {
        let file_type = response.headers.get("type").unwrap().as_str().unwrap();
        match file_type {
            "file" => {
                let content_length = response
                    .headers
                    .get("content-length")
                    .unwrap()
                    .as_u64()
                    .unwrap();
                println!(
                    "INFO: {} is a file with {} bytes",
                    remote_path.to_str().unwrap(),
                    content_length
                );
            }
            "directory" => println!("INFO: {} is a directory", remote_path.to_str().unwrap()),
            _ => println!("Malformed info response"),
        }
    } else {
        println!(
            "Response status code is not OK: {}",
            response.status_code.to_string()
        );
    }

    Ok(())
}

fn main() -> Result<(), Box<dyn Error>> {
    let app = App::new("SSFTP client")
        .version("1.0")
        .author("Kienyew <kienyew@sjtu.edu.cn>")
        .about("client implementing Super Simple File Transfer Protocol")
        .arg(
            Arg::with_name("host")
                .help("ip and port of server host")
                .value_name("ip:port")
                .takes_value(true)
                .required(true)
                .index(1)
                .validator(socket_addr_validator),
        )
        .subcommand(
            App::new("get")
                .about("Send a GET request")
                .arg(
                    Arg::with_name("remote-path")
                        .help("requested file path on server")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::with_name("local-path")
                        .help("path to save the file on local maching")
                        .required(true)
                        .index(2),
                ),
        )
        .subcommand(
            App::new("dir").about("Send a DIR request").arg(
                Arg::with_name("remote-path")
                    .help("requested file path on server")
                    .required(true)
                    .index(1),
            ),
        )
        .subcommand(
            App::new("info").about("Send a INFO request").arg(
                Arg::with_name("remote-path")
                    .help("requested file path on server")
                    .required(true)
                    .index(1),
            ),
        );

    let matches = app.get_matches();
    let socket_addr: SocketAddr = matches.value_of("host").unwrap().parse().unwrap();
    let ssftp_client = SSFTPClient::new(socket_addr.ip(), socket_addr.port());

    match matches.subcommand() {
        ("get", Some(sub)) => run_get(ssftp_client, sub),
        ("dir", Some(sub)) => run_dir(ssftp_client, sub),
        ("info", Some(sub)) => run_info(ssftp_client, sub),
        _ => {
            println!("No subcommand, try --help");
            Ok(())
        }
    }?;

    Ok(())
}
