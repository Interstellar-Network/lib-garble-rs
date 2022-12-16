#[cfg(all(not(feature = "std"), feature = "sgx"))]
use http_req_sgx as http_req;
#[cfg(feature = "std")]
use http_req_std as http_req;

use alloc::string::String;
use alloc::vec::Vec;
use core::time::Duration;
use http_req::error as http_req_error;
use http_req::request::{Method, RequestBuilder};
use http_req::uri::Uri;
use serde::Deserialize;
use serde_json::from_str;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use snafu::prelude::*;
use std::format;
use std::net::TcpStream;
use std::string::ToString;

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

/// eg: "{"Name":"TODO_path","Hash":"QmUjBgZpddDdKZkAFszLyrX2YkBLPKLmkKWJFsU1fTcJWo","Size":"36"}"
/// cf https://github.com/ferristseng/rust-ipfs-api/blob/master/ipfs-api-prelude/src/response/add.rs
#[serde_as]
#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub struct IpfsAddResponse {
    pub name: String,
    pub hash: String,
    #[serde_as(as = "DisplayFromStr")]
    pub size: usize,
}

/// IPFS add
/// cf https://docs.ipfs.tech/reference/kubo/rpc/#api-v0-add
/// and https://github.com/ferristseng/rust-ipfs-api/blob/master/ipfs-api-prelude/src/request/add.rs
///
/// param root_uri: eg "http://localhost:5001"
pub fn ipfs_add(root_uri: &str, body: &[u8]) -> Result<IpfsAddResponse, IpfsError> {
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

    // https://github.com/mikedilger/formdata/blob/master/src/lib.rs
    let body_bytes = b"--boundary\r\n\
                      Content-Disposition: form-data; name=\"file\"; filename=\"TODO_path\"\r\n\
                      Content-Type: application/octet-stream\r\n\
                      \r\n\
                      TODO_content1\r\n\
                      TODO_content2\r\n\
                      --boundary--";

    // TODO(interstellar)???
    // let body_bytes = body_bytes.replace("\n", "\r\n");
    // .header("Content-Type", "application/octet-stream")
    //Add header `Connection: Close`
    let mut request: RequestBuilder = RequestBuilder::new(&addr);
    request.timeout(Some(Duration::from_millis(1000)));
    // TODO(interstellar) keep-alive? is it needed?
    // TODO(interstellar)
    request.header("Content-Type", "multipart/form-data;boundary=\"boundary\"");
    request.header("Content-Length", &body_bytes.len().to_string());
    request.method(Method::POST);
    // TODO(interstellar)
    request.body(body_bytes);
    let result = request.send(&mut stream, &mut writer);

    // println!("{}", String::from_utf8_lossy(&writer));
    // println!("Status: {} {}", response.status_code(), response.reason());

    match result {
        Ok(response) => {
            let status_code = response.status_code();
            if status_code.is_success() {
                let add_response: IpfsAddResponse = serde_json::from_slice(&writer).unwrap();
                Ok(add_response)
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
