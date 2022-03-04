use rsvici::{error::Category, Client};

use pretty_assertions::assert_eq;
use serde::Deserialize;
use tokio_test::io::Builder;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct Version {
    daemon: String,
    version: String,
    sysname: String,
    release: String,
    machine: String,
}

#[tokio::test]
async fn request() {
    #[rustfmt::skip]
    let mock_stream = Builder::new()
        .write(&[
            // header
            0, 0, 0, 9,
            // packet type
            0, 7, b'v', b'e', b'r', b's', b'i', b'o', b'n',
        ])
        .read(&[
            // header
            0, 0, 0, 100,
            // packet type
            1,
            // daemon = charon-systemd
            3, 6, b'd', b'a', b'e', b'm', b'o', b'n', 0, 14, b'c', b'h', b'a', b'r', b'o', b'n', b'-', b's', b'y', b's', b't', b'e', b'm', b'd',
            // version = 5.9.5
            3, 7, b'v', b'e', b'r', b's', b'i', b'o', b'n', 0, 5, b'5', b'.', b'9', b'.', b'5',
            // sysname = Linux
            3, 7, b's', b'y', b's', b'n', b'a', b'm', b'e', 0, 5, b'L', b'i', b'n', b'u', b'x',
            // release = 5.16.16-arch1-1
            3, 7, b'r', b'e', b'l', b'e', b'a', b's', b'e', 0, 15, b'5', b'.', b'1', b'6', b'.', b'1', b'6', b'-', b'a', b'r', b'c', b'h', b'1', b'-', b'1',
            // machine = x86_64
            3, 7, b'm', b'a', b'c', b'h', b'i', b'n', b'e', 0, 6, b'x', b'8', b'6', b'_', b'6', b'4',
        ])
        .build();

    let mut client = Client::new(mock_stream);

    let actual: Version = client.request("version", ()).await.unwrap();
    assert_eq!(
        actual,
        Version {
            daemon: "charon-systemd".to_string(),
            version: "5.9.5".to_string(),
            sysname: "Linux".to_string(),
            release: "5.16.16-arch1-1".to_string(),
            machine: "x86_64".to_string(),
        }
    );
}

#[tokio::test]
async fn request_unknown_cmd() {
    #[rustfmt::skip]
    let mock_stream = Builder::new()
        .write(&[
            // header
            0, 0, 0, 14,
            // packet type
            0, 12, b'n', b'o', b'n', b'-', b'e', b'x', b'i', b's', b't', b'i', b'n', b'g',
        ])
        .read(&[
            // header
            0, 0, 0, 1,
            // packet type
            2,
        ])
        .build();

    let mut client = Client::new(mock_stream);

    let actual = client.request::<(), Version>("non-existing", ()).await.unwrap_err();
    assert_eq!(actual.classify(), Category::UnknownCmd);
}
