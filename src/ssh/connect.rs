use std::{
    net::{TcpStream, ToSocketAddrs},
    time::Duration,
};

use ssh2::{MethodType, Session};
use ssh2_config::HostParams;

use crate::{check, util::read_from_stdin};

pub(super) fn try_connection(host: &str) -> Option<TcpStream> {
    host.to_socket_addrs()
        .expect("To be handled")
        .find_map(|addr| {
            let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(30));
            match stream {
                Ok(stream) => Some(stream),
                Err(_) => None,
            }
        })
}

pub(super) fn authenticate(session: &mut Session, params: &HostParams) {
    let user = params
        .user
        .clone()
        .unwrap_or_else(|| read_from_stdin(false, "Username :"));

    let password = read_from_stdin(true, &format!("[{}] Password: ", user));

    if let Err(err) = session.userauth_password(&user, &password) {
        panic!("Authentication failed: {}", err);
    }
}

// Used mostly the same logic to https://github.com/veeso/ssh2-config/blob/main/examples/client.rs
pub(super) fn configure_session(session: &mut Session, params: &HostParams) {
    if let Some(compress) = params.compression {
        session.set_compress(compress);
    }
    if params.tcp_keep_alive.unwrap_or(false) && params.server_alive_interval.is_some() {
        let interval = params.server_alive_interval.unwrap().as_secs() as u32;
        session.set_keepalive(true, interval);
    }

    macro_rules! report_if_fail {
        ($op: expr, $err: expr) => {{
            let operation = { $op };
            check!(operation.is_ok(), "{}: {}", $err, operation.unwrap_err());
        }};
    }

    // algos
    if let Some(algos) = params.kex_algorithms.as_deref() {
        report_if_fail!(
            session.method_pref(MethodType::Kex, algos.join(",").as_str()),
            "Could not set KEX algorithms"
        );
    }
    if let Some(algos) = params.host_key_algorithms.as_deref() {
        report_if_fail!(
            session.method_pref(MethodType::HostKey, algos.join(",").as_str()),
            "Could not set host key algorithms"
        );
    }
    if let Some(algos) = params.ciphers.as_deref() {
        report_if_fail!(
            session.method_pref(MethodType::CryptCs, algos.join(",").as_str()),
            "Could not set crypt algorithms (client-server)"
        );
        report_if_fail!(
            session.method_pref(MethodType::CryptSc, algos.join(",").as_str()),
            "Could not set crypt algorithms (server-client)"
        );
    }
    if let Some(algos) = params.mac.as_deref() {
        report_if_fail!(
            session.method_pref(MethodType::MacCs, algos.join(",").as_str()),
            "Could not set MAC algorithms (client-server)"
        );
        report_if_fail!(
            session.method_pref(MethodType::MacSc, algos.join(",").as_str()),
            "Could not set MAC algorithms (server-client)"
        )
    }
}
