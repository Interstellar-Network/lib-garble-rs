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
    #[snafu(display("uri error: {}", msg))]
    UriError { msg: String },
    #[snafu(display("tcp stream error: {}", msg))]
    TcpStreamError { msg: String },
    #[snafu(display("serde error: {}", err))]
    DeserializationError { err: serde_json::Error },
    #[snafu(display("utf8 error: {}", err))]
    Utf8Error { err: std::string::FromUtf8Error },
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

#[derive(Deserialize, Debug)]
// #[serde(transparent)]
pub struct IpfsCatResponse(Vec<u8>);

// https://github.com/mikedilger/formdata/blob/master/src/lib.rs
// WARNING: DO NOT use "\n" as end of line: it MUST be escaped(hence '\' in this example)
// let body_bytes = b"--boundary\r\n\
//                 Content-Disposition: form-data; name=\"file\"; filename=\"TODO_path\"\r\n\
//                 Content-Type: application/octet-stream\r\n\
//                 \r\n\
//                 TODO_content1\r\n\
//                 TODO_content2\r\n\
//                 --boundary--";
pub const MULTIPART_NEW_LINE: &[u8] = b"\r\n";
pub const MULTIPART_BOUNDARY: &[u8] = b"--boundary";
pub const MULTIPART_CONTENT_DISPOSITION: &[u8] =
    b"Content-Disposition: form-data; name=\"file\"; filename=\"TODO_path\"";
pub const MULTIPART_CONTENT_TYPE: &[u8] = b"Content-Type: application/octet-stream";

pub struct IpfsClient {
    // This is NOT a Uri b/c it would require keep a ref to the underlying &str; ie Uri<'a>
    root_uri: String,
    stream: TcpStream,
}

#[cfg(feature = "std")]
fn parse_uri<'a>(uri_str: &'a str) -> Result<Uri<'a>, IpfsError> {
    // Parse uri and assign it to variable `addr`
    // TODO(interstellar) why do we get "the trait `FromStr` is not implemented for `Uri<'_>`" in either SGX or STD???
    let addr: Uri = Uri::try_from(uri_str).map_err(|err| IpfsError::UriError {
        msg: format!("invalid uri ({}) : {} ", uri_str, err),
    })?;

    Ok(addr)
}

#[cfg(all(not(feature = "std"), feature = "sgx"))]
fn parse_uri(uri_str: &str) -> Result<Uri, IpfsError> {
    let addr: Uri = uri_str.parse().map_err(|err| IpfsError::UriError {
        msg: format!("invalid uri: {}", uri_str),
    })?;

    Ok(addr)
}

impl IpfsClient {
    pub fn new(root_uri: &str) -> Result<Self> {
        let api_uri = format!("{}{}", root_uri, VERSION_PATH_V0);
        let addr = parse_uri(&api_uri)?;

        //Connect to remote host
        let stream = TcpStream::connect((
            addr.host().ok_or_else(|| IpfsError::UriError {
                msg: format!("invalid host: {}", addr),
            })?,
            addr.corr_port(),
        ))
        .map_err(|err| IpfsError::TcpStreamError {
            msg: err.to_string(),
        })?;

        // Open secure connection over TlsStream, because of `addr` (https)
        // TODO(interstellar) IPFS support https
        // let mut stream = tls::Config::default()
        //     .connect(addr.host().unwrap_or(""), stream)
        //     .unwrap();

        Ok(IpfsClient {
            root_uri: api_uri,
            stream,
        })
    }

    /// IPFS add
    /// cf https://docs.ipfs.tech/reference/kubo/rpc/#api-v0-add
    /// and https://github.com/ferristseng/rust-ipfs-api/blob/master/ipfs-api-prelude/src/request/add.rs
    ///
    /// param root_uri: eg "http://localhost:5001"
    pub fn ipfs_add(&mut self, body: &[u8]) -> Result<IpfsAddResponse, IpfsError> {
        // TODO(interstellar) avoid copying
        let multipart_start = [
            MULTIPART_BOUNDARY,
            MULTIPART_NEW_LINE,
            MULTIPART_CONTENT_DISPOSITION,
            MULTIPART_NEW_LINE,
            MULTIPART_CONTENT_TYPE,
            MULTIPART_NEW_LINE,
            // Space b/w "headers" and "body"
            MULTIPART_NEW_LINE,
        ]
        .concat();
        // No need for a new line at the end
        let body_bytes = [
            multipart_start.as_slice(),
            body,
            MULTIPART_NEW_LINE,
            MULTIPART_BOUNDARY,
            b"--",
        ]
        .concat();

        let full_uri_str = format!("{}/add", self.root_uri);
        let full_uri = parse_uri(&full_uri_str)?;
        let mut request = new_request(&full_uri)?;
        request.header("Content-Type", "multipart/form-data;boundary=\"boundary\"");
        request.header("Content-Length", &body_bytes.len().to_string());
        // TODO(interstellar)
        request.body(&body_bytes);

        let mut writer = Vec::new();
        send_request(&mut self.stream, &mut writer, request)
    }

    /// https://docs.ipfs.tech/reference/kubo/rpc/#api-v0-cat
    ///
    /// NOTE: "This endpoint returns a `text/plain` response body."
    pub fn ipfs_cat(&mut self, ipfs_hash: &str) -> Result<Vec<u8>, IpfsError> {
        // TODO(interstellar) args: &offset=<value>&length=<value>&progress=false
        let full_uri_str = format!("{}/cat?arg={}", self.root_uri, ipfs_hash);
        let full_uri = parse_uri(&full_uri_str)?;
        let request = new_request(&full_uri)?;

        // TODO(interstellar) can we make it work using eg IpfsCatResponse, #serde(transparent)? etc?
        let mut writer = Vec::new();
        send_request_raw_response(&mut self.stream, &mut writer, request)
    }
}

/// response is a JSON struct
fn send_request<'a, ResponseType: Deserialize<'a>>(
    stream: &mut TcpStream,
    writer: &'a mut Vec<u8>,
    request: RequestBuilder,
) -> Result<ResponseType, IpfsError> {
    let result = request.send(stream, writer);

    match result {
        Ok(response) => {
            let status_code = response.status_code();
            if status_code.is_success() {
                let add_response: ResponseType = serde_json::from_slice(writer)
                    .map_err(|err| IpfsError::DeserializationError { err })?;
                Ok(add_response)
            } else {
                Err(IpfsError::HttpError {
                    // TODO(interstellar) remove clone
                    msg: String::from_utf8(writer.clone())
                        .map_err(|err| IpfsError::Utf8Error { err })?,
                    code: u16::from(response.status_code()),
                })
            }
        }
        Err(err) => Err(IpfsError::ResponseError { err: err }),
    }
}

/// response is raw data
// TODO(interstellar) can we combine send_request and send_request_raw_response
fn send_request_raw_response<'a>(
    stream: &mut TcpStream,
    writer: &'a mut Vec<u8>,
    request: RequestBuilder,
) -> Result<Vec<u8>, IpfsError> {
    let result = request.send(stream, writer);

    match result {
        Ok(response) => {
            let status_code = response.status_code();
            if status_code.is_success() {
                Ok(writer.clone())
            } else {
                Err(IpfsError::HttpError {
                    // TODO(interstellar) remove clone
                    msg: String::from_utf8(writer.clone())
                        .map_err(|err| IpfsError::Utf8Error { err })?,
                    code: u16::from(response.status_code()),
                })
            }
        }
        Err(err) => Err(IpfsError::ResponseError { err: err }),
    }
}

fn new_request<'a>(full_uri: &'a Uri) -> Result<RequestBuilder<'a>> {
    // TODO(interstellar) keep-alive? is it needed? or Close?
    let mut request: RequestBuilder = RequestBuilder::new(full_uri);
    // TODO(interstellar) timeout from new()
    request.timeout(Some(Duration::from_millis(1000)));
    request.method(Method::POST);

    Ok(request)
}
