use std::{
    io::Write,
    path::Path,
    sync::Arc,
    time::Duration, net::{ToSocketAddrs, TcpStream},
};

use ssh2::{Session, MethodType, Channel};
use ssh2_config::HostParams;
use tokio::sync::Mutex;

use crate::{
    check,
    progress::ProgressView,
     util::read_from_stdin,
};

pub mod exec;
pub mod transfer;

pub struct SSHSession(Arc<Mutex<Session>>);
impl SSHSession {
    pub fn open(hostname: &str, params: &HostParams) -> Self {
        let host = params.host_name.as_deref().unwrap_or(hostname);
        let host = if host.contains(':') {
            check!(
                params.port.is_none(),
                "Port {} is ignored, because hostname seems to contain port (it has ':')",
                params.port.unwrap()
            );
            host.to_string()
        } else {
            let port = params.port.unwrap_or(22);
            format!("{}:{}", host, port)
        };

        let stream = ProgressView::with("Connecting to the host..", |mut progress| {
            let stream = try_connection(&host).expect("Could not connect to the host");
            progress.success(Some(&format!(
                "Connected to {}",
                stream
                    .peer_addr()
                    .map(|addr| addr.to_string())
                    .unwrap_or("[host]".to_string())
            )));

            stream
        });

        let mut session = ProgressView::with("Configuring the session...", |mut progress| {
            let mut session = Session::new().expect("Could not create session");
            configure_session(&mut session, params);
            session.set_tcp_stream(stream);
            session.handshake().unwrap();
            progress.success(None);

            session
        });

        authenticate(&mut session, params);

        println!("✅ Connected to the remote server");

        if let Some(banner) = session.banner() {
            println!("----------------------------------");
            println!("{}", banner);
            println!("----------------------------------");
        }

        Self(Arc::new(Mutex::new(session)))
    }

    pub(self) async fn create_exec_channel(&self,) -> Channel {
        self.0.clone().lock().await.channel_session().unwrap()
    }

    pub(self) async fn transfer_scp(&self, dest: &Path, content: &[u8]) {
        let session = self.0.lock().await;

        let mut scp_session = session
            .scp_send(dest, 0o644, content.len() as u64, None)
            .unwrap();
        scp_session.write_all(content).unwrap();
        scp_session.send_eof().unwrap();
        scp_session.wait_eof().unwrap();
        scp_session.close().unwrap();
        scp_session.wait_close().unwrap();
    }
}

fn try_connection(host: &str) -> Option<TcpStream> {
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

fn authenticate(session: &mut Session, params: &HostParams) {
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
fn configure_session(session: &mut Session, params: &HostParams) {
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