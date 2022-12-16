mod common;
use ipfs_api_backend_hyper::IpfsApi;
use lib_garble_rs::ipfs::ipfs_add;
use libp2p::futures::TryStreamExt;

#[test]
fn test_ipfs_add() {
    let (foreign_node, ipfs_client) = common::foreign_ipfs::run_ipfs_in_background();
    // let ipfs_server_multiaddr = format!("/ip4/127.0.0.1/tcp/{}", foreign_node.api_port);
    let ipfs_server_multiaddr = format!("http://localhost:{}", foreign_node.api_port);

    // AZaz
    let content = &[65u8, 90, 97, 122];
    let res = ipfs_add(&ipfs_server_multiaddr, content);

    assert!(res.is_ok());

    let add_response = res.unwrap();

    // Compare using the official client; API call = IPFS cat
    let skcd_buf = tokio_test::block_on({
        ipfs_client
            .cat(&add_response.hash)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
    })
    .unwrap();
    let skcd_buf_std = String::from_utf8(skcd_buf).unwrap();
    assert_eq!(skcd_buf_std, "AZaz");
}
