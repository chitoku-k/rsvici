use indexmap::{indexmap, IndexMap};

use rsvici::Client;

use futures_util::stream::TryStreamExt;
use pretty_assertions::assert_eq;
use serde::Deserialize;
use tokio_test::io::Builder;

type Conns = IndexMap<String, Conn>;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Conn {
    local_addrs: Vec<String>,
    remote_addrs: Vec<String>,
    version: String,
    reauth_time: u64,
    rekey_time: u64,
}

#[tokio::test]
async fn stream_request() {
    #[rustfmt::skip]
    let mock_stream = Builder::new()
        .write(&[
            // header
            0, 0, 0, 11,
            // packet type
            3, 9, b'l', b'i', b's', b't', b'-', b'c', b'o', b'n', b'n',
        ])
        .read(&[
            // header
            0, 0, 0, 1,
            // packet type
            5,
        ])
        .write(&[
            // header
            0, 0, 0, 12,
            // packet type
            0, 10, b'l', b'i', b's', b't', b'-', b'c', b'o', b'n', b'n', b's',
        ])
        .read(&[
            // header
            0, 0, 0, 116,
            // packet type
            7, 9, b'l', b'i', b's', b't', b'-', b'c', b'o', b'n', b'n',
            // conn-0
            1, 6, b'c', b'o', b'n', b'n', b'-', b'0',
            // local_addrs
            4, 11, b'l', b'o', b'c', b'a', b'l', b'_', b'a', b'd', b'd', b'r', b's',
            // %any
            5, 0, 4, b'%', b'a', b'n', b'y',
            // local_addrs end
            6,
            // remote_addrs
            4, 12, b'r', b'e', b'm', b'o', b't', b'e', b'_', b'a', b'd', b'd', b'r', b's',
            // %any
            5, 0, 4, b'%', b'a', b'n', b'y',
            // remote_addrs end
            6,
            // version = IKEv1/2
            3, 7, b'v', b'e', b'r', b's', b'i', b'o', b'n', 0, 7, b'I', b'K', b'E', b'v', b'1', b'/', b'2',
            // reauth_time = 0
            3, 11, b'r', b'e', b'a', b'u', b't', b'h', b'_', b't', b'i', b'm', b'e', 0, 1, b'0',
            // rekey_time = 14400
            3, 10, b'r', b'e', b'k', b'e', b'y', b'_', b't', b'i', b'm', b'e', 0, 5, b'1', b'4', b'4', b'0', b'0',
            // conn-0 end
            2,
        ])
        .read(&[
            // header
            0, 0, 0, 114,
            // packet type
            7, 9, b'l', b'i', b's', b't', b'-', b'c', b'o', b'n', b'n',
            // conn-1
            1, 6, b'c', b'o', b'n', b'n', b'-', b'1',
            // local_addrs
            4, 11, b'l', b'o', b'c', b'a', b'l', b'_', b'a', b'd', b'd', b'r', b's',
            // %any
            5, 0, 4, b'%', b'a', b'n', b'y',
            // local_addrs end
            6,
            // remote_addrs
            4, 12, b'r', b'e', b'm', b'o', b't', b'e', b'_', b'a', b'd', b'd', b'r', b's',
            // %any
            5, 0, 4, b'%', b'a', b'n', b'y',
            // remote_addrs end
            6,
            // version = IKEv2
            3, 7, b'v', b'e', b'r', b's', b'i', b'o', b'n', 0, 5, b'I', b'K', b'E', b'v', b'2',
            // reauth_time = 0
            3, 11, b'r', b'e', b'a', b'u', b't', b'h', b'_', b't', b'i', b'm', b'e', 0, 1, b'0',
            // rekey_time = 86400
            3, 10, b'r', b'e', b'k', b'e', b'y', b'_', b't', b'i', b'm', b'e', 0, 5, b'8', b'6', b'4', b'0', b'0',
            // conn-0 end
            2,
        ])
        .read(&[
            // header
            0, 0, 0, 1,
            // packet type
            1,
        ])
        .write(&[
            // header
            0, 0, 0, 11,
            // packet type
            4, 9, b'l', b'i', b's', b't', b'-', b'c', b'o', b'n', b'n',
        ])
        .read(&[
            // header
            0, 0, 0, 1,
            // packet type
            5,
        ])
        .build();

    let mut client = Client::new(mock_stream);

    let stream = client.stream_request::<(), Conns>("list-conns", "list-conn", ());
    let actual: Vec<_> = stream.try_collect().await.unwrap();
    assert_eq!(
        actual,
        vec![
            indexmap! {
                "conn-0".to_string() => Conn {
                    local_addrs: vec![
                        "%any".to_string(),
                    ],
                    remote_addrs: vec![
                        "%any".to_string(),
                    ],
                    version: "IKEv1/2".to_string(),
                    reauth_time: 0,
                    rekey_time: 14400,
                },
            },
            indexmap! {
                "conn-1".to_string() => Conn {
                    local_addrs: vec![
                        "%any".to_string(),
                    ],
                    remote_addrs: vec![
                        "%any".to_string(),
                    ],
                    version: "IKEv2".to_string(),
                    reauth_time: 0,
                    rekey_time: 86400,
                },
            },
        ]
    );
}
