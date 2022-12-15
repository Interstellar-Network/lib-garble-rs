#[cfg(all(not(feature = "std"), feature = "sgx"))]
use http_req_sgx as http_req;
#[cfg(feature = "std")]
use http_req_std as http_req;

use alloc::string::String;
use alloc::vec::Vec;
use core::time::Duration;
use http_req::error as http_req_error;
use http_req::request::copy_exact;
use http_req::request::copy_with_timeout;
use http_req::request::{Method, RequestBuilder};
use http_req::response::Response;
use http_req::response::StatusCode;
use http_req::tls;
use http_req::uri::Uri;
use snafu::prelude::*;
use std::format;
use std::io::copy;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::string::ToString;
use std::time::Instant;

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

    // let body_bytes = r#"
    // --===============0688100289\r\n
    // Content-Disposition: form-data; name="path"\r\n
    // \r\n
    // TODO_path
    // --===============0688100289\r\n
    // Content-Disposition: form-data; name="html-data"; filename="user.html"\r\n
    // Content-Type: application/octet-stream\r\n
    // \r\n
    // TODO_body\r\n
    // --===============0688100289--\r\n
    //     "#;

    // let body_bytes = r#"--===============0688100289\r\n
    // Content-Disposition: form-data; name="file"; filename="requireoptions.txt"\r\n
    // Content-Type: text/plain\r\n
    // \r\n
    // Pillow
    // pyusb
    // wxPython
    // ezdxf
    // opencv-python-headless
    // \r\n--===============0688100289--\r\n"#;

    // https://stackoverflow.com/questions/31406022/how-is-an-http-multipart-content-length-header-value-calculated
    //     let body_bytes = r#"--===============0688100289==\r\n
    // Content-type: form-data\r\n
    // \r\n
    // {"title": "test-multipart.txt", "parents": [{"id":"0B09i2ZH5SsTHTjNtSS9QYUZqdTA"}], "properties": [{"kind": "drive#property", "key": "cloudwrapper", "value": "true"}]}\r\n
    // --===============0688100289==\r\n
    // Content-type: form-data\r\n
    // \r\n
    // We're testing multipart uploading!\r\n
    // --===============0688100289==--
    // "#;

    //     let body_bytes = r#"--===============0688100289==

    // --boundary
    // Content-Disposition: form-data; path="field1"

    // value1
    // --boundary
    // Content-Disposition: form-data; name="field2"; filename="example.txt"

    // value2
    // --boundary--
    // "#;

    // https://github.com/mikedilger/formdata/blob/master/src/lib.rs
    let body_bytes = b"--boundary\r\n\
                      Content-Disposition: form-data; name=\"field1\"\r\n\
                      \r\n\
                      data1\r\n\
                      --boundary\r\n\
                      Content-Disposition: form-data; name=\"field2\"; filename=\"image.gif\"\r\n\
                      Content-Type: image/gif\r\n\
                      \r\n\
                      This is a file\r\n\
                      with two lines\r\n\
                      --boundary\r\n\
                      Content-Disposition: form-data; name=\"field3\"; filename=\"file.txt\"\r\n\
                      \r\n\
                      This is a file\r\n\
                      --boundary--";

    // TODO(interstellar)???
    // let body_bytes = body_bytes.replace("\n", "\r\n");

    //Add header `Connection: Close`
    let mut request: RequestBuilder = RequestBuilder::new(&addr);
    request.timeout(Some(Duration::from_millis(1000)));
    // .header("Connection", "Close")
    // .header(
    //     "Content-Disposition",
    //     "form-data; name=\"file\"; filename=\"folderName%2Ffile.txt\"",
    // )
    // .header("Content-Type", "application/octet-stream")
    // TODO(interstellar)
    request.header("Content-Type", "multipart/form-data;boundary=\"boundary\"");
    // .header(
    //     "Content-Type",
    //     "multipart/form-data; boundary================0688100289",
    // )
    // request.header("Content-Length", &999.to_string());
    request.header("Content-Length", &body_bytes.len().to_string());
    request.method(Method::POST);
    // TODO(interstellar)
    request.body(body_bytes);
    // .write_msg(&mut stream, body_bytes)
    let result = request.send(&mut stream, &mut writer);

    // let result = raw_send(
    //     &request,
    //     None,
    //     Method::POST,
    //     &mut stream,
    //     &mut writer,
    //     body_bytes.as_bytes(),
    // );

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

/// Based on "pub fn send<T, U>(&self, stream: &mut T, writer: &mut U) -> Result<Response, error::Error>"
/// but simplified.
///
/// We NEED this to send multipart/form-data apparently...
/// Else only "body_part" is sent and the get a 500 "EOF" from IPFS API.
fn raw_send<T, U>(
    request: &RequestBuilder,
    timeout: Option<Duration>,
    method: Method,
    stream: &mut T,
    writer: &mut U,
    body: &[u8],
) -> Result<Response, http_req_error::Error>
where
    T: Write + Read,
    U: Write,
{
    ////////////////////////////////////////////////////////////////////////////

    request.write_msg(stream, &request.parse_msg()).unwrap();

    let head_deadline = match timeout {
        Some(t) => Instant::now() + t,
        None => Instant::now() + Duration::from_secs(360),
    };
    let (res, body_part) = request.read_head(stream, head_deadline)?;

    if method == Method::HEAD {
        return Ok(res);
    }

    ////////////////////////////////////////////////////////////////////////////

    // if let Some(v) = res.headers().get("Transfer-Encoding") {
    //     if *v == "chunked" {
    //         let mut dechunked = crate::chunked::Reader::new(body_part.as_slice().chain(stream));

    //         if let Some(timeout) = self.timeout {
    //             let deadline = Instant::now() + timeout;
    //             copy_with_timeout(&mut dechunked, writer, deadline)?;
    //         } else {
    //             io::copy(&mut dechunked, writer)?;
    //         }

    //         return Ok(res);
    //     }
    // }

    // writer.write_all(&request)?;

    ////////////////////////////////////////////////////////////////////////////

    writer.write_all(&body)?;

    if let Some(timeout) = timeout {
        let deadline = Instant::now() + timeout;
        copy_with_timeout(stream, writer, deadline)?;
    } else {
        // TODO(interstellar)
        let num_bytes = res.content_len().unwrap_or(0);
        // let num_bytes = body.len();

        if num_bytes > 0 {
            // TODO(interstellar)
            copy_exact(stream, writer, num_bytes - body_part.len())?;
            // copy_exact(stream, writer, num_bytes)?;
        } else {
            copy(stream, writer)?;
        }
    }

    Ok(res)
}
