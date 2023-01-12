mod common;
use crate::common::foreign_ipfs::ForeignNode;
use crossbeam_utils::thread;
use ipfs_api_backend_hyper::IpfsApi;
use ipfs_client_http_req::IpfsClient;
use libp2p::futures::TryStreamExt;
use std::io::Cursor;

fn setup_ipfs() -> (IpfsClient, ipfs_api_backend_hyper::IpfsClient, ForeignNode) {
    let (foreign_node, ipfs_reference_client) = common::foreign_ipfs::run_ipfs_in_background();
    // let ipfs_server_multiaddr = format!("/ip4/127.0.0.1/tcp/{}", foreign_node.api_port);
    let ipfs_server_multiaddr = format!("http://localhost:{}", foreign_node.api_port);
    let ipfs_internal_client = IpfsClient::new(&ipfs_server_multiaddr).unwrap();

    (ipfs_internal_client, ipfs_reference_client, foreign_node)
}

fn test_ipfs_add_aux<'a>(
    ipfs_internal_client: &'a IpfsClient,
    ipfs_reference_client: &'a ipfs_api_backend_hyper::IpfsClient,
) {
    // AZaz
    let content = &[65u8, 90, 97, 122];

    let res = ipfs_internal_client.ipfs_add(content);

    let add_response = res.unwrap();

    // Compare using the official client; API call = IPFS cat
    let skcd_buf = tokio_test::block_on({
        ipfs_reference_client
            .cat(&add_response.hash)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
    })
    .unwrap();
    let res_str = String::from_utf8(skcd_buf).unwrap();
    assert_eq!(res_str, "AZaz");

    // let ipfs_internal_client: &'b IpfsClient = ipfs_internal_client;
    // let ipfs_reference_client: &'b ipfs_api_backend_hyper::IpfsClient = ipfs_reference_client;

    // (ipfs_internal_client, ipfs_reference_client)
}

#[test]
fn test_ipfs_add() {
    let (ipfs_internal_client, ipfs_reference_client, _foreign_node) = setup_ipfs();
    test_ipfs_add_aux(&ipfs_internal_client, &ipfs_reference_client);
}

#[test]
fn test_ipfs_cat() {
    let (ipfs_internal_client, ipfs_reference_client, _foreign_node) = setup_ipfs();

    // AZaz
    let content = &[65u8, 90, 97, 122];
    let cursor = Cursor::new(content);
    let ipfs_server_response = tokio_test::block_on(ipfs_reference_client.add(cursor)).unwrap();

    let res = ipfs_internal_client.ipfs_cat(&ipfs_server_response.hash);

    let res = res.unwrap();

    let res_str = String::from_utf8(res).unwrap();
    assert_eq!(res_str, "AZaz");
}

// TODO(interstellar) Test with multiple requests to make sure write/stream are reusable
#[test]
fn test_ipfs_multiple_adds() {
    let (ipfs_internal_client, ipfs_reference_client, _foreign_node) = setup_ipfs();
    test_ipfs_add_aux(&ipfs_internal_client, &ipfs_reference_client);
    test_ipfs_add_aux(&ipfs_internal_client, &ipfs_reference_client);
    test_ipfs_add_aux(&ipfs_internal_client, &ipfs_reference_client);
}

/// https://rust-lang.github.io/api-guidelines/interoperability.html#types-are-send-and-sync-where-possible-c-send-sync
#[test]
fn require_send_sync() {
    fn assert_send<T: Send>() {}
    assert_send::<IpfsClient>();

    fn assert_sync<T: Sync>() {}
    assert_sync::<IpfsClient>();
}

#[test]
fn test_ipfs_thread_safe_adds() {
    let (ipfs_internal_client, ipfs_reference_client, _foreign_node) = setup_ipfs();

    // IMPORTANT: MUST use https://docs.rs/crossbeam-utils/latest/crossbeam_utils/thread/index.html b/c
    // std::thread CAN NOT borrow from the stack
    thread::scope(|s| {
        for _ in 1..10 {
            let ipfs_internal_client_ref = &ipfs_internal_client;
            let ipfs_reference_client_ref = &ipfs_reference_client;
            s.spawn(move |_| {
                test_ipfs_add_aux(ipfs_internal_client_ref, ipfs_reference_client_ref);
            });
        }
    })
    .unwrap();
}
