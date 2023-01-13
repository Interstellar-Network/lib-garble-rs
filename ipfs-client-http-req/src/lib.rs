// #![no_std]
#![cfg_attr(not(feature = "std"), no_std)]
#![deny(elided_lifetimes_in_paths)]

extern crate alloc;

use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;
use core::time::Duration;
use serde::Deserialize;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use snafu::prelude::*;
use std::format;

/// cf https://github.com/ferristseng/rust-ipfs-api/blob/master/ipfs-api-prelude/src/from_uri.rs#L17
const VERSION_PATH_V0: &str = "/api/v0";

#[derive(Debug, Snafu)]
pub enum IpfsError {
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
    #[snafu(display("stream io error: {}", err))]
    IoStreamError { err: std::io::Error },
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

/// IpfsClient using http_req
/// Compatible with no_std/sgx
///
/// Only support a SUBSET of the API; namely ADD and CAT for now
///
/// NOTE: for thread safety reasons, the underlying "stream" is NOT kept around
/// cf commented-out code relating to it if needed in the future.
/// In which case you will probably need to replace Request for RequestBuilder
/// As a bonus it avoids trying to connect in "new" which can be useful.
pub struct IpfsClient {
    // This is NOT a Uri b/c it would require keep a ref to the underlying &str; ie Uri<'a>
    root_uri: String,
    // TODO(interstellar) thread safety: or something else?
    // stream: Arc<RwLock<TcpStream>>,
    // stream: TcpStream,
}

impl IpfsClient {
    pub fn new(root_uri: &str) -> Result<Self> {
        let api_uri = format!("{}{}", root_uri, VERSION_PATH_V0);

        // let addr = parse_uri(&api_uri)?;

        //Connect to remote host
        // let stream = TcpStream::connect((
        //     addr.host().ok_or_else(|| IpfsError::UriError {
        //         msg: format!("invalid host: {}", addr),
        //     })?,
        //     addr.corr_port(),
        // ))
        // .map_err(|err| IpfsError::TcpStreamError {
        //     msg: err.to_string(),
        // })?;

        // Open secure connection over TlsStream, because of `addr` (https)
        // TODO(interstellar) IPFS support https
        // let mut stream = tls::Config::default()
        //     .connect(addr.host().unwrap_or(""), stream)
        //     .unwrap();

        Ok(IpfsClient { root_uri: api_uri })
    }

    /// IPFS add
    /// cf https://docs.ipfs.tech/reference/kubo/rpc/#api-v0-add
    /// and https://github.com/ferristseng/rust-ipfs-api/blob/master/ipfs-api-prelude/src/request/add.rs
    ///
    /// param root_uri: eg "http://localhost:5001"
    pub fn ipfs_add(&self, body: &[u8]) -> Result<IpfsAddResponse, IpfsError> {
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
        let (response_body, content_type) = ocw_common::sp_offchain_fetch_from_remote_grpc_web(
            Some(body_bytes.into()),
            &full_uri_str,
            ocw_common::RequestMethod::Post,
            Some(ocw_common::ContentType::MultipartFormData),
            Duration::from_millis(2000),
        )
        .map_err(|err| IpfsError::HttpError {
            msg: "TODO".to_string(),
            code: 500,
        })?;

        Ok(serde_json::from_slice(response_body.as_ref())
            .map_err(|err| IpfsError::DeserializationError { err })?)
    }

    /// https://docs.ipfs.tech/reference/kubo/rpc/#api-v0-cat
    ///
    /// NOTE: "This endpoint returns a `text/plain` response body."
    pub fn ipfs_cat(&self, ipfs_hash: &str) -> Result<Vec<u8>, IpfsError> {
        // TODO(interstellar) args: &offset=<value>&length=<value>&progress=false
        let full_uri_str = format!("{}/cat?arg={}", self.root_uri, ipfs_hash);
        let (response_body, content_type) = ocw_common::sp_offchain_fetch_from_remote_grpc_web(
            None,
            &full_uri_str,
            ocw_common::RequestMethod::Post,
            None,
            Duration::from_millis(2000),
        )
        .map_err(|err| IpfsError::HttpError {
            msg: "TODO".to_string(),
            code: 500,
        })?;

        Ok(response_body.to_vec())
    }
}
