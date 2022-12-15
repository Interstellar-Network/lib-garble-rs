#[cfg(all(not(feature = "std"), feature = "sgx"))]
use http_req_sgx as http_req;
#[cfg(feature = "std")]
use http_req_std as http_req;

use alloc::vec::Vec;
use http_req::error as http_req_error;
use http_req::request::{Method, RequestBuilder};
use http_req::tls;
use http_req::uri::Uri;
use snafu::prelude::*;
use std::convert::TryFrom;
use std::net::TcpStream;

#[derive(Debug, Snafu)]
pub enum IpfsError {
    #[snafu(display("response error: {}", err))]
    ResponseError { err: http_req_error::Error },
}

type Result<T, E = IpfsError> = std::result::Result<T, E>;

pub fn ipfs_add(uri: &str) -> Result<(), IpfsError> {
    //Parse uri and assign it to variable `addr`
    let addr: Uri = Uri::try_from(uri).map_err(|err| IpfsError::ResponseError { err: err })?;

    //Connect to remote host
    let stream = TcpStream::connect((addr.host().unwrap(), addr.corr_port())).unwrap();

    //Open secure connection over TlsStream, because of `addr` (https)
    let mut stream = tls::Config::default()
        .connect(addr.host().unwrap_or(""), stream)
        .unwrap();

    //Container for response's body
    let mut writer = Vec::new();

    //Add header `Connection: Close`
    let result = RequestBuilder::new(&addr)
        .header("Connection", "Close")
        .method(Method::POST)
        .send(&mut stream, &mut writer);

    // println!("{}", String::from_utf8_lossy(&writer));
    // println!("Status: {} {}", response.status_code(), response.reason());

    match result {
        Ok(response) => Ok(()),
        Err(err) => Err(IpfsError::ResponseError { err: err }),
    }
}
