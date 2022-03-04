use rsvici::Client;

use futures_util::{stream::TryStreamExt, StreamExt};
use pretty_assertions::assert_eq;
use serde::Deserialize;
use tokio_test::io::Builder;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Log {
    group: String,
    level: u32,
    msg: String,
}

#[tokio::test]
async fn subscribe() {
    #[rustfmt::skip]
    let (mock_stream, mut handle) = Builder::new()
        .write(&[
            // header
            0, 0, 0, 5,
            // packet type
            3, 3, b'l', b'o', b'g',
        ])
        .read(&[
            // header
            0, 0, 0, 5,
            // packet type
            5, 3, b'l', b'o', b'g',
        ])
        .read(&[
            // header
            0, 0, 0, 66,
            // packet type
            7, 3, b'l', b'o', b'g',
            // group = IKE
            3, 5, b'g', b'r', b'o', b'u', b'p', 0, 3, b'I', b'K', b'E',
            // level = 1
            3, 5, b'l', b'e', b'v', b'e', b'l', 0, 1, b'1',
            // msg = received FRAGMENTATION vendor ID
            3, 3, b'm', b's', b'g', 0, 32, b'r', b'e', b'c', b'e', b'i', b'v', b'e', b'd', b' ', b'F', b'R', b'A', b'G', b'M', b'E', b'N', b'T', b'A', b'T', b'I', b'O', b'N', b' ', b'v', b'e', b'n', b'd', b'o', b'r', b' ', b'I', b'D',
        ])
        .read(&[
            // header
            0, 0, 0, 56,
            // packet type
            7, 3, b'l', b'o', b'g',
            // group = IKE
            3, 5, b'g', b'r', b'o', b'u', b'p', 0, 3, b'I', b'K', b'E',
            // level = 1
            3, 5, b'l', b'e', b'v', b'e', b'l', 0, 1, b'1',
            // msg = received DPD vendor ID
            3, 3, b'm', b's', b'g', 0, 22, b'r', b'e', b'c', b'e', b'i', b'v', b'e', b'd', b' ', b'D', b'P', b'D', b' ', b'v', b'e', b'n', b'd', b'o', b'r', b' ', b'I', b'D',
        ])
        .build_with_handle();

    let mut client = Client::new(mock_stream);

    {
        let stream = client.subscribe::<Log>("log");
        let actual: Vec<_> = stream.take(2).try_collect().await.unwrap();
        assert_eq!(
            actual,
            vec![
                Log {
                    group: "IKE".to_string(),
                    level: 1,
                    msg: "received FRAGMENTATION vendor ID".to_string(),
                },
                Log {
                    group: "IKE".to_string(),
                    level: 1,
                    msg: "received DPD vendor ID".to_string(),
                },
            ]
        );
    }

    #[rustfmt::skip]
    handle
        .write(&[
            // header
            0, 0, 0, 5,
            // packet type
            4, 3, b'l', b'o', b'g',
        ])
        .read(&[
            // header
            0, 0, 0, 76,
            // packet type
            7, 3, b'l', b'o', b'g',
            // group = IKE
            3, 5, b'g', b'r', b'o', b'u', b'p', 0, 3, b'I', b'K', b'E',
            // level = 1
            3, 5, b'l', b'e', b'v', b'e', b'l', 0, 1, b'1',
            // msg = 192.0.2.1 is itiniating a Main Mode IKE_SA
            3, 3, b'm', b's', b'g', 0, 42, b'1', b'9', b'2', b'.', b'0', b'.', b'2', b'.', b'1', b' ', b'i', b's', b' ', b'i', b't', b'i', b'n', b'i', b'a', b't', b'i', b'n', b'g', b' ', b'a', b' ', b'M', b'a', b'i', b'n', b' ', b'M', b'o', b'd', b'e', b' ', b'I', b'K', b'E', b'_', b'S', b'A', 
        ])
        .read(&[
            // header
            0, 0, 0, 5,
            // packet type
            5,
        ]);
}
