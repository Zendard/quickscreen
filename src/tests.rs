use crate::host;
use std::sync::mpsc;

#[test]
fn host() {
    let (sender, _) = mpsc::channel();
    let (_, receiver) = mpsc::channel();
    host::host(1111, sender, receiver);
}
