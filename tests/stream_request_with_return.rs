use core::option::Option::None;

use rsvici::{Client, Error};

use futures_util::{pin_mut, stream::TryStreamExt};
use pretty_assertions::assert_eq;
use serde::{Deserialize, Serialize};
use tokio_test::io::Builder;

#[derive(Debug, Deserialize, Eq, PartialEq)]
struct ControlLog {
    group: String,
    level: u32,
    #[serde(rename = "ikesa-name")]
    ikesa_name: String,
    #[serde(rename = "ikesa-uniqueid")]
    ikesa_uniqueid: u64,
    msg: String,
}

#[derive(Debug, Serialize)]
struct Initiate {
    ike: String,
    child: String,
}

#[tokio::test]
async fn stream_request_with_success() {
    #[rustfmt::skip]
    let mock_stream = Builder::new()
        .write(&[
            // header
            0, 0, 0, 13,
            // packet type
            3, 11, b'c', b'o', b'n', b't', b'r', b'o', b'l', b'-', b'l', b'o', b'g',
        ])
        .read(&[
            // header
            0, 0, 0, 1,
            // packet type
            5,
        ])
        .write(&[
            // header
            0, 0, 0, 38,
            // packet type
            0, 8, b'i', b'n', b'i', b't', b'i', b'a', b't', b'e',
            // ike = gw-gw
            3, 3, b'i', b'k', b'e', 0, 5, b'g', b'w', b'-', b'g', b'w',
            // child = net-net
            3, 5, b'c', b'h', b'i', b'l', b'd', 0, 7, b'n', b'e', b't', b'-', b'n', b'e', b't',
        ])
        .read(&[
            // header
            0, 0, 0, 107,
            // packet type
            7, 11, b'c', b'o', b'n', b't', b'r', b'o', b'l', b'-', b'l', b'o', b'g',
            // group = IKE
            3, 5, b'g', b'r', b'o', b'u', b'p', 0, 3, b'I', b'K', b'E',
            // level = 1
            3, 5, b'l', b'e', b'v', b'e', b'l', 0, 1, b'1',
            // ikesa-name = gw-gw
            3, 10, b'i', b'k', b'e', b's', b'a', b'-', b'n', b'a', b'm', b'e', 0, 5, b'g', b'w', b'-', b'g', b'w',
            // ikesa-uniqueid = 12
            3, 14, b'i', b'k', b'e', b's', b'a', b'-', b'u', b'n', b'i', b'q', b'u', b'e', b'i', b'd', 0, 2, b'1', b'2',
            // msg = IKE_SA gw-gw[12] initiated
            3, 3, b'm', b's', b'g', 0, 26, b'I', b'K', b'E', b'_', b'S', b'A', b' ', b'g', b'w', b'-', b'g', b'w', b'[', b'1', b'2', b']', b' ', b'i', b'n', b'i', b't', b'i', b'a', b't', b'e', b'd',
        ])
        .read(&[
            // header
            0, 0, 0, 103,
            // packet type
            7, 11, b'c', b'o', b'n', b't', b'r', b'o', b'l', b'-', b'l', b'o', b'g',
            // group = ENC
            3, 5, b'g', b'r', b'o', b'u', b'p', 0, 3, b'E', b'N', b'C',
            // level = 1
            3, 5, b'l', b'e', b'v', b'e', b'l', 0, 1, b'1',
            // ikesa-name = gw-gw
            3, 10, b'i', b'k', b'e', b's', b'a', b'-', b'n', b'a', b'm', b'e', 0, 5, b'g', b'w', b'-', b'g', b'w',
            // ikesa-uniqueid = 12
            3, 14, b'i', b'k', b'e', b's', b'a', b'-', b'u', b'n', b'i', b'q', b'u', b'e', b'i', b'd', 0, 2, b'1', b'2',
            // msg = AES-CBC-128 negotiated
            3, 3, b'm', b's', b'g', 0, 22, b'A', b'E', b'S', b'-', b'C', b'B', b'C', b'-', b'1', b'2', b'8', b' ', b'n', b'e', b'g', b'o', b't', b'i', b'a', b't', b'e', b'd',
        ])
        .read(&[
            // header
            0, 0, 0, 15,
            // packet type
            1,
            // success = yes
            3, 7, b's', b'u', b'c', b'c', b'e', b's', b's', 0, 3, b'y', b'e', b's',
        ])
        .write(&[
            // header
            0, 0, 0, 13,
            // packet type
            4, 11, b'c', b'o', b'n', b't', b'r', b'o', b'l', b'-', b'l', b'o', b'g',
        ])
        .read(&[
            // header
            0, 0, 0, 1,
            // packet type
            5,
        ])
        .build();

    let mut client = Client::new(mock_stream);
    let initiate = Initiate {
        ike: "gw-gw".to_string(),
        child: "net-net".to_string(),
    };

    let stream = client.stream_request::<Initiate, ControlLog>("initiate", "control-log", initiate);
    let actual: Vec<ControlLog> = stream.try_collect().await.unwrap();

    assert_eq!(
        actual,
        vec![
            ControlLog {
                group: "IKE".to_string(),
                level: 1,
                ikesa_name: "gw-gw".to_string(),
                ikesa_uniqueid: 12,
                msg: "IKE_SA gw-gw[12] initiated".to_string(),
            },
            ControlLog {
                group: "ENC".to_string(),
                level: 1,
                ikesa_name: "gw-gw".to_string(),
                ikesa_uniqueid: 12,
                msg: "AES-CBC-128 negotiated".to_string(),
            },
        ]
    );
}

#[tokio::test]
async fn stream_request_with_failure() {
    #[rustfmt::skip]
    let mock_stream = Builder::new()
        .write(&[
            // header
            0, 0, 0, 13,
            // packet type
            3, 11, b'c', b'o', b'n', b't', b'r', b'o', b'l', b'-', b'l', b'o', b'g',
        ])
        .read(&[
            // header
            0, 0, 0, 1,
            // packet type
            5,
        ])
        .write(&[
            // header
            0, 0, 0, 38,
            // packet type
            0, 8, b'i', b'n', b'i', b't', b'i', b'a', b't', b'e',
            // ike = gw-gw
            3, 3, b'i', b'k', b'e', 0, 5, b'g', b'w', b'-', b'g', b'w',
            // child = net-net
            3, 5, b'c', b'h', b'i', b'l', b'd', 0, 7, b'n', b'e', b't', b'-', b'n', b'e', b't',
        ])
        .read(&[
            // header
            0, 0, 0, 100,
            // packet type
            7, 11, b'c', b'o', b'n', b't', b'r', b'o', b'l', b'-', b'l', b'o', b'g',
            // group = ENC
            3, 5, b'g', b'r', b'o', b'u', b'p', 0, 3, b'E', b'N', b'C',
            // level = 1
            3, 5, b'l', b'e', b'v', b'e', b'l', 0, 1, b'1',
            // ikesa-name = gw-gw
            3, 10, b'i', b'k', b'e', b's', b'a', b'-', b'n', b'a', b'm', b'e', 0, 5, b'g', b'w', b'-', b'g', b'w',
            // ikesa-uniqueid = 12
            3, 14, b'i', b'k', b'e', b's', b'a', b'-', b'u', b'n', b'i', b'q', b'u', b'e', b'i', b'd', 0, 2, b'1', b'2',
            // msg = failed to negotiate
            3, 3, b'm', b's', b'g', 0, 19, b'f', b'a', b'i', b'l', b'e', b'd', b' ', b't', b'o', b' ', b'n', b'e', b'g', b'o', b't', b'i', b'a', b't', b'e',
        ])
        .read(&[
            // header
            0, 0, 0, 55,
            // packet type
            1,
            // success = no
            3, 7, b's', b'u', b'c', b'c', b'e', b's', b's', 0, 2, b'n', b'o',
            // errmsg = child initiation net-net failed
            3, 6, b'e', b'r', b'r', b'm', b's', b'g', 0, 31, b'c', b'h', b'i', b'l', b'd', b' ', b'i', b'n', b'i', b't', b'i', b'a', b't', b'i', b'o', b'n', b' ', b'n', b'e', b't', b'-', b'n', b'e', b't', b' ', b'f', b'a', b'i', b'l', b'e', b'd',
        ])
        .write(&[
            // header
            0, 0, 0, 13,
            // packet type
            4, 11, b'c', b'o', b'n', b't', b'r', b'o', b'l', b'-', b'l', b'o', b'g',
        ])
        .read(&[
            // header
            0, 0, 0, 1,
            // packet type
            5,
        ])
        .build();

    let mut client = Client::new(mock_stream);
    let initiate = Initiate {
        ike: "gw-gw".to_string(),
        child: "net-net".to_string(),
    };

    let stream = client.stream_request::<Initiate, ControlLog>("initiate", "control-log", initiate);

    pin_mut!(stream);

    let mut items: Vec<ControlLog> = Vec::new();
    let mut err: Option<Error> = None;

    loop {
        match stream.try_next().await {
            Ok(Some(item)) => items.push(item),
            Ok(None) => break,
            Err(e) => {
                err = Some(e);
                break;
            },
        }
    }

    assert_eq!(
        items,
        vec![ControlLog {
            group: "ENC".to_string(),
            level: 1,
            ikesa_name: "gw-gw".to_string(),
            ikesa_uniqueid: 12,
            msg: "failed to negotiate".to_string(),
        },]
    );
    assert_eq!(err.map(|e| e.to_string()), Some("command failed: child initiation net-net failed".to_string()));
}

#[tokio::test]
async fn stream_request_with_failure_no_errmsg() {
    #[rustfmt::skip]
    let mock_stream = Builder::new()
        .write(&[
            // header
            0, 0, 0, 13,
            // packet type
            3, 11, b'c', b'o', b'n', b't', b'r', b'o', b'l', b'-', b'l', b'o', b'g',
        ])
        .read(&[
            // header
            0, 0, 0, 1,
            // packet type
            5,
        ])
        .write(&[
            // header
            0, 0, 0, 38,
            // packet type
            0, 8, b'i', b'n', b'i', b't', b'i', b'a', b't', b'e',
            // ike = gw-gw
            3, 3, b'i', b'k', b'e', 0, 5, b'g', b'w', b'-', b'g', b'w',
            // child = net-net
            3, 5, b'c', b'h', b'i', b'l', b'd', 0, 7, b'n', b'e', b't', b'-', b'n', b'e', b't',
        ])
        .read(&[
            // header
            0, 0, 0, 100,
            // packet type
            7, 11, b'c', b'o', b'n', b't', b'r', b'o', b'l', b'-', b'l', b'o', b'g',
            // group = ENC
            3, 5, b'g', b'r', b'o', b'u', b'p', 0, 3, b'E', b'N', b'C',
            // level = 1
            3, 5, b'l', b'e', b'v', b'e', b'l', 0, 1, b'1',
            // ikesa-name = gw-gw
            3, 10, b'i', b'k', b'e', b's', b'a', b'-', b'n', b'a', b'm', b'e', 0, 5, b'g', b'w', b'-', b'g', b'w',
            // ikesa-uniqueid = 12
            3, 14, b'i', b'k', b'e', b's', b'a', b'-', b'u', b'n', b'i', b'q', b'u', b'e', b'i', b'd', 0, 2, b'1', b'2',
            // msg = failed to negotiate
            3, 3, b'm', b's', b'g', 0, 19, b'f', b'a', b'i', b'l', b'e', b'd', b' ', b't', b'o', b' ', b'n', b'e', b'g', b'o', b't', b'i', b'a', b't', b'e',
        ])
        .read(&[
            // header
            0, 0, 0, 14,
            // packet type
            1,
            // success = no
            3, 7, b's', b'u', b'c', b'c', b'e', b's', b's', 0, 2, b'n', b'o',
        ])
        .write(&[
            // header
            0, 0, 0, 13,
            // packet type
            4, 11, b'c', b'o', b'n', b't', b'r', b'o', b'l', b'-', b'l', b'o', b'g',
        ])
        .read(&[
            // header
            0, 0, 0, 1,
            // packet type
            5,
        ])
        .build();

    let mut client = Client::new(mock_stream);
    let initiate = Initiate {
        ike: "gw-gw".to_string(),
        child: "net-net".to_string(),
    };

    let stream = client.stream_request::<Initiate, ControlLog>("initiate", "control-log", initiate);

    pin_mut!(stream);

    let mut items: Vec<ControlLog> = Vec::new();
    let mut err: Option<Error> = None;

    loop {
        match stream.try_next().await {
            Ok(Some(item)) => items.push(item),
            Ok(None) => break,
            Err(e) => {
                err = Some(e);
                break;
            },
        }
    }

    assert_eq!(
        items,
        vec![ControlLog {
            group: "ENC".to_string(),
            level: 1,
            ikesa_name: "gw-gw".to_string(),
            ikesa_uniqueid: 12,
            msg: "failed to negotiate".to_string(),
        },]
    );
    assert_eq!(err.map(|e| e.to_string()), Some("command failed".to_string()));
}
