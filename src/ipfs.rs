#[cfg(all(not(feature = "std"), feature = "sgx"))]
use http_req_sgx as http_req;
#[cfg(feature = "std")]
use http_req_std as http_req;

use alloc::string::String;
use alloc::vec::Vec;
use http_req::error as http_req_error;
use http_req::request::{Method, RequestBuilder};
use http_req::response::StatusCode;
use http_req::tls;
use http_req::uri::Uri;
use snafu::prelude::*;
use std::format;
use std::net::TcpStream;

/// cf https://github.com/ferristseng/rust-ipfs-api/blob/master/ipfs-api-prelude/src/from_uri.rs#L17
const VERSION_PATH_V0: &str = "/api/v0";

#[derive(Debug, Snafu)]
pub enum IpfsError {
    #[snafu(display("response error: {}", err))]
    ResponseError { err: http_req_error::Error },
    #[snafu(display("http error[{}]: {}", code, msg))]
    HttpError { msg: String, code: u16 },
}

type Result<T, E = IpfsError> = std::result::Result<T, E>;

/// IPFS add
/// cf https://docs.ipfs.tech/reference/kubo/rpc/#api-v0-add
/// and https://github.com/ferristseng/rust-ipfs-api/blob/master/ipfs-api-prelude/src/request/add.rs
///
/// param root_uri: eg "http://localhost:5001"
pub fn ipfs_add(root_uri: &str, body: &[u8]) -> Result<Vec<u8>, IpfsError> {
    let uri = format!("{}{}/add", root_uri, VERSION_PATH_V0);
    let uri: &str = &uri;

    // Parse uri and assign it to variable `addr`
    // TODO(interstellar) why do we get "the trait `FromStr` is not implemented for `Uri<'_>`" in either SGX or STD???
    #[cfg(feature = "std")]
    let addr: Uri = Uri::try_from(uri).map_err(|err| IpfsError::ResponseError { err: err })?;
    #[cfg(all(not(feature = "std"), feature = "sgx"))]
    let addr: Uri = uri.parse().unwrap();

    //Connect to remote host
    let mut stream = TcpStream::connect((addr.host().unwrap(), addr.corr_port())).unwrap();

    // Open secure connection over TlsStream, because of `addr` (https)
    // TODO(interstellar) IPFS support https
    // let mut stream = tls::Config::default()
    //     .connect(addr.host().unwrap_or(""), stream)
    //     .unwrap();

    //Container for response's body
    let mut writer = Vec::new();

    //Add header `Connection: Close`
    let result = RequestBuilder::new(&addr)
        .header("Connection", "Close")
        .header(
            "Content-Disposition",
            "form-data; name=\"file\"; filename=\"folderName%2Ffile.txt\"",
        )
        .header("Content-Type", "application/octet-stream")
        .method(Method::POST)
        .body(body)
        .send(&mut stream, &mut writer);

    // println!("{}", String::from_utf8_lossy(&writer));
    // println!("Status: {} {}", response.status_code(), response.reason());

    match result {
        Ok(response) => {
            let status_code = response.status_code();
            if status_code.is_success() {
                Ok(writer)
            } else {
                Err(IpfsError::HttpError {
                    msg: String::from_utf8(writer).unwrap(),
                    code: u16::from(response.status_code()),
                })
            }
        }
        Err(err) => Err(IpfsError::ResponseError { err: err }),
    }
}
