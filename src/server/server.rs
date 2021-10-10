use ssftp::utils::sanitize_request_path;
use ssftp::StatusCode;

use std::error::Error;
use std::fs::{File, Metadata, ReadDir};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use colored::*; // cli output coloring
use serde_json::json;
use threadpool;

pub struct SSFTPServer {
    config: Arc<Mutex<ServerConfig>>,
    listener: TcpListener,
    thread_pool: threadpool::ThreadPool,
}

struct ServerConfig {
    serve_dir: PathBuf,
}

/// Contains all needed information of an incoming request for preparation of response
#[derive(Debug)]
enum Request {
    Get(PathBuf),
    Info(PathBuf),
    Dir(PathBuf),
}

/// A pending response, before any performance of huge read & write
#[derive(Debug)]
enum PendingResponse {
    Get(File),
    Info(Metadata),
    Dir(ReadDir),
    Error(StatusCode),
    BadRequest,
}

/// Parse the raw request and return the type of request.
fn get_request(stream: &mut TcpStream) -> Result<Request, &str> {
    let mut raw_request = vec![];
    if BufReader::new(stream)
        .read_until('\n' as u8, &mut raw_request)
        .is_err()
    {
        return Err("read tcp stream error");
    }

    raw_request.pop();

    let raw_request = String::from_utf8_lossy(&raw_request[..]);

    println!("Raw request: {}", raw_request);

    let mut iterator = raw_request.splitn(2, ' ').into_iter();
    match (iterator.next(), iterator.next()) {
        (Some(method), Some(path)) => match &method.to_uppercase()[..] {
            "GET" => Ok(Request::Get(PathBuf::from(path))),
            "INFO" => Ok(Request::Info(PathBuf::from(path))),
            "DIR" => Ok(Request::Dir(PathBuf::from(path))),
            _ => Err("bad method"),
        },
        _ => Err("bad request"),
    }
}

impl SSFTPServer {
    /// Create a server
    /// # Arguments
    /// * `socket_addr`: the ip and port to listen on
    /// * `serve_dir`: the root path of directory to serve file.
    /// * `thread_count`: number of thread to use.
    pub fn new(socket_addr: SocketAddr, serve_dir: &Path, thread_count: usize) -> Self {
        let serve_dir = PathBuf::from(serve_dir);
        let listener = TcpListener::bind(socket_addr).unwrap();
        let thread_pool = threadpool::Builder::new().num_threads(thread_count).build();

        SSFTPServer {
            config: Arc::new(Mutex::new(ServerConfig { serve_dir })),
            listener,
            thread_pool,
        }
    }

    /// Block the program and start listening.
    pub fn start(&self) {
        for stream in self.listener.incoming() {
            match stream {
                Ok(stream) => {
                    println!(
                        "\nIncoming connection from {}",
                        (if let Ok(addr) = stream.peer_addr() {
                            addr.to_string()
                        } else {
                            "?".into()
                        })
                        .yellow()
                        .bold()
                    );
                    let config = Arc::clone(&self.config);
                    self.thread_pool.execute(|| {
                        if let Err(err) = handle_client(config, stream) {
                            println!("Error occured while handling a client. {}", err);
                        }
                    })
                }

                Err(err) => println!("Error occured while handling a stream. {}", err),
            }
        }
    }
}

fn handle_client(
    config: Arc<Mutex<ServerConfig>>,
    mut stream: TcpStream,
) -> Result<(), Box<dyn Error>> {
    let request = get_request(&mut stream);

    let response;
    if let Ok(request) = request {
        response = prepare_response(config, &request)?;
    } else {
        response = PendingResponse::BadRequest;
    }

    perform_response(stream, response)?;
    Ok(())
}

fn prepare_response(
    config: Arc<Mutex<ServerConfig>>,
    request: &Request,
) -> Result<PendingResponse, Box<dyn Error>> {
    match request {
        Request::Get(path) | Request::Info(path) | Request::Dir(path) => {
            if !sanitize_request_path(path) {
                return Ok(PendingResponse::BadRequest);
            }

            let path_in_fs = if let Ok(config) = config.lock() {
                config.serve_dir.join(path.strip_prefix("/")?)
            } else {
                return Err("Multithread lock error".into());
            };

            if !path_in_fs.exists() {
                return Ok(PendingResponse::Error(StatusCode::NotExist));
            }

            match request {
                Request::Get(_) => {
                    if let Ok(file) = File::open(path_in_fs) {
                        if !file.metadata()?.is_file() {
                            Ok(PendingResponse::Error(StatusCode::NotFile))
                        } else {
                            Ok(PendingResponse::Get(file))
                        }
                    } else {
                        Ok(PendingResponse::Error(StatusCode::ServerError))
                    }
                }

                Request::Info(_) => {
                    if let Ok(metadata) = path_in_fs.metadata() {
                        Ok(PendingResponse::Info(metadata))
                    } else {
                        Ok(PendingResponse::Error(StatusCode::ServerError))
                    }
                }

                Request::Dir(_) => {
                    if let Ok(metadata) = path_in_fs.metadata() {
                        if !metadata.is_dir() {
                            return Ok(PendingResponse::Error(StatusCode::NotDirectory));
                        }
                    }

                    if let Ok(read_dir) = path_in_fs.read_dir() {
                        Ok(PendingResponse::Dir(read_dir))
                    } else {
                        Ok(PendingResponse::Error(StatusCode::ServerError))
                    }
                }
            }
        }
    }
}

/// Perform a pending response given by `response`, write through the internet.
/// Act as a a dispatcher function.
fn perform_response(stream: TcpStream, response: PendingResponse) -> Result<(), Box<dyn Error>> {
    use PendingResponse::*;
    match response {
        Get(file) => perform_get_response(stream, file),
        Info(metadata) => perform_info_response(stream, metadata),
        Dir(read_dir) => perform_dir_response(stream, read_dir),
        Error(status_code) => perform_error_response(stream, status_code),
        BadRequest => perform_bad_request_response(stream),
    }
}

fn perform_get_response(stream: TcpStream, file: File) -> Result<(), Box<dyn Error>> {
    println!("Performing {} response", "GET".green().bold());

    let content_length = file.metadata()?.len();
    let headers = json!({
        "content-length": content_length,
    });

    let mut writer = BufWriter::new(stream);
    writer.write(StatusCode::OK.to_string().as_bytes())?;
    writer.write(&['\n' as u8])?;
    writer.write(headers.to_string().as_bytes())?;
    writer.write(&['\n' as u8])?;

    let mut reader = BufReader::new(file);
    io::copy(&mut reader, &mut writer)?;
    Ok(())
}

fn perform_info_response(stream: TcpStream, metadata: Metadata) -> Result<(), Box<dyn Error>> {
    println!("Performing {} response", "INFO".blue().bold());

    let headers = if metadata.is_dir() {
        json!({ "type": "directory" })
    } else {
        json!({ "type": "file", "content-length": metadata.len() })
    };

    let mut writer = BufWriter::new(stream);
    writer.write(StatusCode::OK.to_string().as_bytes())?;
    writer.write(&['\n' as u8])?;
    writer.write(headers.to_string().as_bytes())?;
    writer.write(&['\n' as u8])?;
    Ok(())
}

fn perform_dir_response(stream: TcpStream, read_dir: ReadDir) -> Result<(), Box<dyn Error>> {
    println!("Performing {} response", "DIR".magenta().bold());
    let entries: Vec<String> = read_dir
        .filter_map(|entry| {
            entry.ok().and_then(|entry| {
                entry.file_type().ok().and_then(|ft| {
                    if ft.is_dir() {
                        entry
                            .file_name()
                            .into_string()
                            .ok()
                            .and_then(|dirname| Some(dirname + "/"))
                    } else {
                        entry.file_name().into_string().ok()
                    }
                })
            })
        })
        .collect();

    let response_message = entries.join("\n");
    let mut writer = BufWriter::new(stream);
    let content_length = response_message.len();
    let headers = json!({
        "content-length": content_length,
        "count": entries.len(),
    });

    writer.write(StatusCode::OK.to_string().as_bytes())?;
    writer.write(&['\n' as u8])?;
    writer.write(headers.to_string().as_bytes())?;
    writer.write(&['\n' as u8])?;
    writer.write(response_message.as_bytes())?;
    Ok(())
}

fn perform_bad_request_response(stream: TcpStream) -> Result<(), Box<dyn Error>> {
    println!("Performing {} response", "BAD REQUEST".red().bold());
    let mut writer = BufWriter::new(stream);
    writer.write(StatusCode::BadRequest.to_string().as_bytes())?;
    writer.write(b"\n{}\n")?;
    Ok(())
}

fn perform_error_response(
    stream: TcpStream,
    status_code: StatusCode,
) -> Result<(), Box<dyn Error>> {
    println!("Performing {} response", "ERROR".red().bold());
    let mut writer = BufWriter::new(stream);
    writer.write(status_code.to_string().as_bytes())?;
    writer.write(b"\n{}\n")?;
    Ok(())
}
