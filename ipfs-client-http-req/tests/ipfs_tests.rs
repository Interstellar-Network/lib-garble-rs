use ipfs_client_http_req::IpfsClient;
use parking_lot::RwLock;
use sp_core::offchain::testing;
use sp_core::offchain::testing::OffchainState;
use sp_core::offchain::OffchainWorkerExt;
use std::io::Cursor;
use std::sync::Arc;
use tests_utils::foreign_ipfs;
use tests_utils::foreign_ipfs::IpfsApi;
use tests_utils::foreign_ipfs::IpfsClient as IpfsReferenceClient;

fn setup_ipfs() -> (IpfsClient, IpfsReferenceClient, foreign_ipfs::ForeignNode) {
    let (foreign_node, ipfs_reference_client) = foreign_ipfs::run_ipfs_in_background(None);
    // let ipfs_server_multiaddr = format!("/ip4/127.0.0.1/tcp/{}", foreign_node.api_port);
    let ipfs_server_multiaddr = format!("http://127.0.0.1:{}", foreign_node.api_port);
    let ipfs_internal_client = IpfsClient::new(&ipfs_server_multiaddr).unwrap();

    (ipfs_internal_client, ipfs_reference_client, foreign_node)
}

/// Setup a OffchainWorkerExt so that we can make http request with "t.execute_with"
fn new_test_ext() -> (sp_io::TestExternalities, Arc<RwLock<OffchainState>>) {
    let (offchain, state) = testing::TestOffchainExt::new();

    let mut t = sp_io::TestExternalities::default();
    t.register_extension(OffchainWorkerExt::new(offchain));
    (t, state)
}

fn mock_ipfs_add_response(state: &mut testing::OffchainState, api_port: u16, body_bytes: &[u8]) {
    state.expect_request(testing::PendingRequest {
        method: "POST".into(),
        uri: format!("http://127.0.0.1:{}/api/v0/add", api_port),
        // cf "fn decode_rpc_json" for the expected format
        response: Some(br#"{"Name":"TODO_path","Hash":"QmUjBgZpddDdKZkAFszLyrX2YkBLPKLmkKWJFsU1fTcJWo","Size":"36"}"#.to_vec()),
        sent: true,
        headers: vec![(
            "Content-Type".into(),
            "multipart/form-data;boundary=\"boundary\"".into(),
        )],
        // MUST match MyTestCallbackMock
        // But it adds the whole "multipart" boundaries etc
        body: ocw_common::new_multipart_body_bytes(body_bytes),
        response_headers: vec![("content-type".into(), "text/plain".into())],
        ..Default::default()
    });
}

fn mock_ipfs_cat_response(
    state: &mut testing::OffchainState,
    api_port: u16,
    ipfs_cid: &str,
    file_bytes: &[u8],
) {
    state.expect_request(testing::PendingRequest {
        method: "POST".into(),
        uri: format!("http://127.0.0.1:{}/api/v0/cat?arg={}", api_port, ipfs_cid),
        // cf "fn decode_rpc_json" for the expected format
        response: Some(file_bytes.to_vec()),
        sent: true,
        headers: vec![],
        response_headers: vec![("content-type".into(), "text/plain".into())],
        ..Default::default()
    });
}

#[test]
fn test_ipfs_add_and_cat_ok() {
    let (ipfs_internal_client, _ipfs_reference_client, foreign_node) = setup_ipfs();
    let (mut t, state) = new_test_ext();

    // MOCK ipfs_internal_client "ipfs_add"
    let content = &[65u8, 90, 97, 122]; // AZaz
    mock_ipfs_add_response(&mut state.write(), foreign_node.api_port, content);
    mock_ipfs_cat_response(
        &mut state.write(),
        foreign_node.api_port,
        "QmUjBgZpddDdKZkAFszLyrX2YkBLPKLmkKWJFsU1fTcJWo",
        content,
    );

    let add_response = t.execute_with(|| {
        let res = ipfs_internal_client.ipfs_add(content);

        res.unwrap()
    });

    // CAT using the official client
    // ...
    //             ipfs_reference_client
    //                 .cat(&add_response.hash)
    //                 .map_ok(|chunk| chunk.to_vec())
    //                 .try_concat()
    //                 .await
    //         })
    //         .unwrap()
    // ...
    // FAIL: never returns; even with tokio::test and await, etc
    // CAT using internal client
    let skcd_buf = t.execute_with(|| {
        let res = ipfs_internal_client.ipfs_cat(&add_response.hash);

        res.unwrap()
    });

    let res_str = String::from_utf8(skcd_buf).unwrap();
    assert_eq!(res_str, "AZaz");
}

#[tokio::test]
async fn test_ipfs_cat_ok() {
    let (ipfs_internal_client, ipfs_reference_client, foreign_node) = setup_ipfs();
    let (mut t, state) = new_test_ext();

    // ADD using the official client
    let content = &[65u8, 90, 97, 122]; // AZaz
    let cursor = Cursor::new(content);
    let ipfs_server_response = ipfs_reference_client.add(cursor).await.unwrap();

    // MOCK ipfs_internal_client "ipfs_cat"
    mock_ipfs_cat_response(
        &mut state.write(),
        foreign_node.api_port,
        &ipfs_server_response.hash,
        content,
    );

    t.execute_with(|| {
        let res = ipfs_internal_client.ipfs_cat(&ipfs_server_response.hash);

        let res = res.unwrap();

        let res_str = String::from_utf8(res).unwrap();
        assert_eq!(res_str, "AZaz");
    });
}

/// https://rust-lang.github.io/api-guidelines/interoperability.html#types-are-send-and-sync-where-possible-c-send-sync
#[test]
fn require_send_sync() {
    fn assert_send<T: Send>() {}
    assert_send::<IpfsClient>();

    fn assert_sync<T: Sync>() {}
    assert_sync::<IpfsClient>();
}

// TODO re-add; but this fail with "called outside of an Externalities-provided environment."
// probably b/c crossbeam thread/scope?
// #[test]
// fn test_ipfs_thread_safe_adds() {
//     let (ipfs_internal_client, _ipfs_reference_client, foreign_node) = setup_ipfs();
//     let (mut t, state) = new_test_ext();

//     // IMPORTANT: MUST use https://docs.rs/crossbeam-utils/latest/crossbeam_utils/thread/index.html b/c
//     // std::thread CAN NOT borrow from the stack
//     t.execute_with(|| {
//         thread::scope(|s| {
//             for i in 1..10 {
//                 let ipfs_internal_client_ref = &ipfs_internal_client;
//                 let foreign_node_ref = &foreign_node;

//                 s.spawn(move |_| {
//                     ipfs_internal_client_ref.ipfs_add(&[0, i]);
//                 });
//             }
//         })
//         .unwrap();
//     });
// }
