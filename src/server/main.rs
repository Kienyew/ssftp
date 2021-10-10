mod server;
use server::SSFTPServer;
use ssftp::utils::socket_addr_validator;

use std::net::SocketAddr;
use std::path::Path;

// cli arguments
use clap::{App, Arg};

use colored::*;

fn main() {
    // cli app
    let app = App::new("server")
        .about("Run the SSFTP server")
        .arg(
            Arg::with_name("host")
                .help("IP address and port to bind on")
                .value_name("ip:port")
                .required(true)
                .validator(socket_addr_validator)
                .index(1),
        )
        .arg(
            Arg::with_name("serve_dir")
                .help("path of serving directory")
                .value_name("path")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::with_name("thread")
                .help("number of thread to open")
                .value_name("thread-count")
                .default_value("8")
                .long("thread")
                .validator(|s| {
                    if s.parse::<usize>().is_err() {
                        Err("Invalid thread count".into())
                    } else {
                        Ok(())
                    }
                }),
        );

    let matches = app.get_matches();
    let socket_addr: SocketAddr = matches.value_of("host").unwrap().parse().unwrap();
    let serve_dir = Path::new(matches.value_of("serve_dir").unwrap());
    let thread_count = matches
        .value_of("thread")
        .unwrap()
        .parse::<usize>()
        .unwrap();

    let server = SSFTPServer::new(socket_addr, serve_dir, thread_count);
    println!(
        "Server starts serving at {}",
        socket_addr.to_string().yellow().bold()
    );
    server.start();
}
