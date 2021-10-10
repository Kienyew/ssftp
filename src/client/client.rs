use ssftp::utils::sanitize_request_path;
use ssftp::StatusCode;

use std::error::Error;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream};
use std::path::Path;

use serde_json;

pub struct SSFTPClient {
    server_addr: SocketAddr,
}

pub struct Response {
    pub status_code: StatusCode,
    pub headers: serde_json::Value,
    pub payload_stream: Box<dyn Read>,
}

impl SSFTPClient {
    pub fn new(host: IpAddr, port: u16) -> Self {
        SSFTPClient {
            server_addr: SocketAddr::new(host, port),
        }
    }

    /// Read and parse status code from raw request stream.
    fn read_status_code(
        &self,
        stream: &mut BufReader<TcpStream>,
    ) -> Result<StatusCode, Box<dyn Error>> {
        let mut line = String::new();
        stream.read_line(&mut line)?;
        Ok(match &line.trim_end().to_uppercase()[..] {
            "OK" => StatusCode::OK,
            "NOT-EXIST" => StatusCode::NotExist,
            "NOT-FILE" => StatusCode::NotFile,
            "NOT-DIRECTORY" => StatusCode::NotDirectory,
            "SERVER-ERROR" => StatusCode::ServerError,
            "BAD-REQUEST" => StatusCode::BadRequest,
            _ => return Err("Unknown status code from response".into()),
        })
    }

    /// Read and parse headers from raw request stream.
    /// Must called after the status code have been read from stream.
    fn read_headers(
        &self,
        stream: &mut BufReader<TcpStream>,
    ) -> Result<serde_json::Value, Box<dyn Error>> {
        let mut line = String::new();
        stream.read_line(&mut line)?;
        Ok(serde_json::from_str(line.trim_end())?)
    }

    /// Send a raw request, return a tcp stream
    fn connect_and_send(&self, request: &[u8]) -> Result<BufReader<TcpStream>, Box<dyn Error>> {
        let mut stream = TcpStream::connect(self.server_addr)?;
        stream.write(request)?;
        Ok(BufReader::new(stream))
    }

    /// send a raw request, read and parse the status code and headers, leaving only payload untouched.
    fn general_request(&self, request: &[u8]) -> Result<Response, Box<dyn Error>> {
        let mut stream = self.connect_and_send(request)?;
        let status_code = self.read_status_code(&mut stream)?;
        let headers = self.read_headers(&mut stream)?;
        Ok(Response {
            status_code,
            headers,
            payload_stream: Box::new(stream),
        })
    }

    /// send a get request and return the response
    pub fn get(&self, path: &Path) -> Result<Response, Box<dyn Error>> {
        if !sanitize_request_path(path) {
            return Err("SSFTPClient::get: bad request path".into());
        }

        let request = format!("GET {}\n", path.to_str().unwrap());
        self.general_request(request.as_bytes())
    }

    /// send an info request and return the response
    pub fn info(&self, path: &Path) -> Result<Response, Box<dyn Error>> {
        if !sanitize_request_path(path) {
            return Err("SSFTPClient::get: bad request path".into());
        }

        let request = format!("INFO {}\n", path.to_str().unwrap());
        self.general_request(request.as_bytes())
    }

    /// send a dir request and return the response
    pub fn dir(&self, path: &Path) -> Result<Response, Box<dyn Error>> {
        if !sanitize_request_path(path) {
            return Err("SSFTPClient::get: bad request path".into());
        }

        let request = format!("DIR {}\n", path.to_str().unwrap());
        self.general_request(request.as_bytes())
    }
}
