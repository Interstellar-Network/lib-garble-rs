mod common;
use ipfs_api_backend_hyper::IpfsApi;
use lib_garble_rs::ipfs::IpfsClient;
use libp2p::futures::TryStreamExt;
use std::io::Cursor;

#[test]
fn test_ipfs_add() {
    let (foreign_node, ipfs_server_client) = common::foreign_ipfs::run_ipfs_in_background();
    // let ipfs_server_multiaddr = format!("/ip4/127.0.0.1/tcp/{}", foreign_node.api_port);
    let ipfs_server_multiaddr = format!("http://localhost:{}", foreign_node.api_port);

    // AZaz
    let content = &[65u8, 90, 97, 122];
    let mut ipfs_client = IpfsClient::new(&ipfs_server_multiaddr).unwrap();
    let res = ipfs_client.ipfs_add(content);

    assert!(res.is_ok());

    let add_response = res.unwrap();

    // Compare using the official client; API call = IPFS cat
    let skcd_buf = tokio_test::block_on({
        ipfs_server_client
            .cat(&add_response.hash)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
    })
    .unwrap();
    let res_str = String::from_utf8(skcd_buf).unwrap();
    assert_eq!(res_str, "AZaz");
}

#[test]
fn test_ipfs_cat() {
    let (foreign_node, ipfs_server_client) = common::foreign_ipfs::run_ipfs_in_background();
    // let ipfs_server_multiaddr = format!("/ip4/127.0.0.1/tcp/{}", foreign_node.api_port);
    let ipfs_server_multiaddr = format!("http://localhost:{}", foreign_node.api_port);

    // AZaz
    let content = &[65u8, 90, 97, 122];
    let cursor = Cursor::new(content);
    let ipfs_server_response = tokio_test::block_on(ipfs_server_client.add(cursor)).unwrap();

    let mut ipfs_client = IpfsClient::new(&ipfs_server_multiaddr).unwrap();
    let res = ipfs_client.ipfs_cat(&ipfs_server_response.hash);

    assert!(res.is_ok());

    let res_data = res.unwrap();

    let res_str = String::from_utf8(res_data).unwrap();
    assert_eq!(res_str, "AZaz");
}

// TODO(interstellar) test with multiple requests to make sure write/stream are reusable
