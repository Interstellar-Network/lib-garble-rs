mod common;
use ipfs_api_backend_hyper::IpfsApi;
use lib_garble_rs::ipfs::ipfs_add;
use libp2p::futures::TryStreamExt;

#[test]
fn test_ipfs_add() {
    let (foreign_node, ipfs_client) = common::foreign_ipfs::run_ipfs_in_background();
    // let ipfs_server_multiaddr = format!("/ip4/127.0.0.1/tcp/{}", foreign_node.api_port);
    let ipfs_server_multiaddr = format!("http://localhost:{}", foreign_node.api_port);

    let content = &[0u8, 42, 1];
    let res = ipfs_add(&ipfs_server_multiaddr, content);

    assert!(res.is_ok());

    let res_bytes = res.unwrap();
    let res_str = String::from_utf8(res_bytes).unwrap();
    let skcd_buf = tokio_test::block_on({
        ipfs_client
            .cat(&res_str)
            .map_ok(|chunk| chunk.to_vec())
            .try_concat()
    });
}
