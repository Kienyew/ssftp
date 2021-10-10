use std::net::SocketAddr;
use std::path::{Component, Path};

/// Check the validity of path given in a client request.
/// # Examples
/// ```
/// assert_eq!(sanitize_request_path(Path::new("/a/valid/request/path")), true);
/// assert_eq!(sanitize_request_path(Path::new("/a/valid/request/dir/path/")), true);
/// assert_eq!(sanitize_request_path(Path::new("/an/invalid/../path")), true);
/// ```
pub fn sanitize_request_path(path: &Path) -> bool {
    path.has_root() && path.components().all(|part| part != Component::ParentDir)
}

pub fn socket_addr_validator(s: String) -> Result<(), String> {
    if s.parse::<SocketAddr>().is_ok() {
        Ok(())
    } else {
        Err("Invalid format of IP address".into())
    }
}
