//! Container healthcheck: `scratch` images have no shell or curl.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::ExitCode;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(3);

/// Requests `/healthz` from the local server; success if any byte comes back.
pub fn run(listen: SocketAddr) -> ExitCode {
    if probe(listen).is_ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn probe(listen: SocketAddr) -> std::io::Result<()> {
    // A wildcard bind address is not connectable; use loopback on its port.
    let target = if listen.ip().is_unspecified() {
        SocketAddr::from(([127, 0, 0, 1], listen.port()))
    } else {
        listen
    };
    let mut stream = TcpStream::connect_timeout(&target, TIMEOUT)?;
    stream.set_read_timeout(Some(TIMEOUT))?;
    stream.set_write_timeout(Some(TIMEOUT))?;
    stream.write_all(b"GET /healthz HTTP/1.0\r\n\r\n")?;
    stream.read_exact(&mut [0u8; 1])
}
