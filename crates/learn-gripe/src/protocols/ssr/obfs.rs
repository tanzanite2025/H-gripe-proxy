//! SSR obfuscation (transport-level disguise) layer.

use anyhow::Result;

use super::crypto::random_bytes;

/// SSR obfuscation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SsrObfs {
    Plain,
    HttpSimple,
    Tls12TicketAuth,
}

/// Obfuscation layer state. Wraps the first packet in a disguise (HTTP GET /
/// TLS Client Hello) and passes subsequent packets through.
pub(super) enum ObfsState {
    Plain,
    HttpSimple(HttpSimpleState),
    Tls12TicketAuth(Tls12TicketAuthState),
}

impl ObfsState {
    pub(super) fn new(obfs: SsrObfs, server: &str, port: u16, obfs_param: &str) -> Self {
        match obfs {
            SsrObfs::Plain => ObfsState::Plain,
            SsrObfs::HttpSimple => ObfsState::HttpSimple(HttpSimpleState::new(server, port, obfs_param)),
            SsrObfs::Tls12TicketAuth => ObfsState::Tls12TicketAuth(Tls12TicketAuthState::new(server, obfs_param)),
        }
    }

    /// Encode outgoing data (may wrap the first packet in HTTP/TLS headers).
    pub(super) fn client_encode(&mut self, data: &[u8]) -> Vec<u8> {
        match self {
            ObfsState::Plain => data.to_vec(),
            ObfsState::HttpSimple(s) => s.client_encode(data),
            ObfsState::Tls12TicketAuth(s) => s.client_encode(data),
        }
    }

    /// Decode incoming data (strip HTTP/TLS framing from the first response).
    /// Returns the decoded data.
    pub(super) fn client_decode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        match self {
            ObfsState::Plain => Ok(data.to_vec()),
            ObfsState::HttpSimple(s) => s.client_decode(data),
            ObfsState::Tls12TicketAuth(s) => s.client_decode(data),
        }
    }
}

// -- http_simple ------------------------------------------------------------

/// Disguises the first packet as an HTTP GET request.
pub(super) struct HttpSimpleState {
    host: String,
    port: u16,
    has_sent_header: bool,
    has_recv_header: bool,
    recv_buf: Vec<u8>,
}

impl HttpSimpleState {
    pub(super) fn new(server: &str, port: u16, obfs_param: &str) -> Self {
        let host = if obfs_param.is_empty() {
            server.to_string()
        } else {
            obfs_param.to_string()
        };
        Self {
            host,
            port,
            has_sent_header: false,
            has_recv_header: false,
            recv_buf: Vec::new(),
        }
    }

    pub(super) fn client_encode(&mut self, data: &[u8]) -> Vec<u8> {
        if self.has_sent_header {
            return data.to_vec();
        }
        self.has_sent_header = true;

        let port_str = if self.port == 80 {
            String::new()
        } else {
            format!(":{}", self.port)
        };

        // Encode first ≤64 bytes of data as hex in the URI path.
        let head_size = data.len().min(64);
        let hex_path: String = data[..head_size].iter().map(|b| format!("{b:02x}")).collect();

        let http_header = format!(
            "GET /{hex_path} HTTP/1.1\r\n\
             Host: {host}{port}\r\n\
             User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36\r\n\
             Accept: text/html,application/xhtml+xml,*/*;q=0.8\r\n\
             Accept-Language: en-US,en;q=0.8\r\n\
             Accept-Encoding: gzip, deflate\r\n\
             DNT: 1\r\n\
             Connection: keep-alive\r\n\
             \r\n",
            hex_path = hex_path,
            host = self.host,
            port = port_str,
        );

        let mut out = Vec::with_capacity(http_header.len() + data.len() - head_size);
        out.extend_from_slice(http_header.as_bytes());
        out.extend_from_slice(&data[head_size..]);
        out
    }

    fn client_decode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if self.has_recv_header {
            return Ok(data.to_vec());
        }
        self.recv_buf.extend_from_slice(data);

        // Look for the end of the HTTP response header (\r\n\r\n).
        if let Some(pos) = find_header_end(&self.recv_buf) {
            self.has_recv_header = true;
            let body = self.recv_buf[pos + 4..].to_vec();
            self.recv_buf.clear();
            Ok(body)
        } else {
            Ok(Vec::new()) // need more data
        }
    }
}

/// Find `\r\n\r\n` in the buffer.
fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

// -- tls1.2_ticket_auth -----------------------------------------------------

/// Disguises the first packet as a TLS 1.2 Client Hello with a session ticket.
pub(super) struct Tls12TicketAuthState {
    host: String,
    has_sent_header: bool,
    has_recv_header: bool,
    recv_buf: Vec<u8>,
}

impl Tls12TicketAuthState {
    pub(super) fn new(server: &str, obfs_param: &str) -> Self {
        let host = if obfs_param.is_empty() {
            server.to_string()
        } else {
            obfs_param.to_string()
        };
        Self {
            host,
            has_sent_header: false,
            has_recv_header: false,
            recv_buf: Vec::new(),
        }
    }

    pub(super) fn client_encode(&mut self, data: &[u8]) -> Vec<u8> {
        if self.has_sent_header {
            // Subsequent packets: wrap as TLS Application Data.
            return self.pack_tls_app_data(data);
        }
        self.has_sent_header = true;
        self.build_client_hello(data)
    }

    fn client_decode(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        if self.has_recv_header {
            // Subsequent packets: unwrap TLS Application Data.
            return self.unpack_tls_records(data);
        }
        self.recv_buf.extend_from_slice(data);

        // Look for TLS records. The first response is a Server Hello
        // (type 0x16) followed by Change Cipher Spec (0x14). We skip
        // all TLS handshake records and return Application Data (0x17).
        self.try_parse_server_response()
    }

    /// Build a fake TLS 1.2 Client Hello with the data as a session ticket.
    fn build_client_hello(&self, data: &[u8]) -> Vec<u8> {
        // SNI extension.
        let sni = self.host.as_bytes();
        let sni_ext_len = 5 + sni.len(); // type(1) + name_len(2) + name_list_len(2)

        // Session ticket extension: the actual encrypted data.
        let ticket_data = data;
        let ticket_ext_len = ticket_data.len();

        // Extensions total length.
        let extensions_len = 4 + sni_ext_len + 4 + ticket_ext_len;

        // Client Hello body.
        let mut hello = Vec::with_capacity(128 + extensions_len);
        // Protocol version: TLS 1.2.
        hello.extend_from_slice(&[0x03, 0x03]);
        // Random (32 bytes).
        let mut random = [0u8; 32];
        random_bytes(&mut random);
        hello.extend_from_slice(&random);
        // Session ID length + session ID (32 bytes).
        hello.push(32);
        let mut session_id = [0u8; 32];
        random_bytes(&mut session_id);
        hello.extend_from_slice(&session_id);
        // Cipher suites (2 suites).
        hello.extend_from_slice(&[0x00, 0x04]); // length
        hello.extend_from_slice(&[0xc0, 0x2b]); // TLS_ECDHE_ECDSA_WITH_AES_128_GCM_SHA256
        hello.extend_from_slice(&[0xc0, 0x2f]); // TLS_ECDHE_RSA_WITH_AES_128_GCM_SHA256
        // Compression methods.
        hello.push(0x01);
        hello.push(0x00); // null compression
        // Extensions length.
        hello.extend_from_slice(&(extensions_len as u16).to_be_bytes());
        // SNI extension (type 0x0000).
        hello.extend_from_slice(&[0x00, 0x00]); // ext type
        hello.extend_from_slice(&((sni_ext_len) as u16).to_be_bytes());
        hello.extend_from_slice(&((sni_ext_len - 2) as u16).to_be_bytes()); // list len
        hello.push(0x00); // host name type
        hello.extend_from_slice(&(sni.len() as u16).to_be_bytes());
        hello.extend_from_slice(sni);
        // Session ticket extension (type 0x0023).
        hello.extend_from_slice(&[0x00, 0x23]); // ext type
        hello.extend_from_slice(&(ticket_ext_len as u16).to_be_bytes());
        hello.extend_from_slice(ticket_data);

        // Wrap in TLS handshake (Client Hello = 0x01).
        let mut handshake = Vec::with_capacity(4 + hello.len());
        handshake.push(0x01); // Client Hello
        // 3-byte length.
        let hl = hello.len();
        handshake.push((hl >> 16) as u8);
        handshake.push((hl >> 8) as u8);
        handshake.push(hl as u8);
        handshake.extend_from_slice(&hello);

        // Wrap in TLS record (Handshake = 0x16).
        let mut record = Vec::with_capacity(5 + handshake.len());
        record.push(0x16); // content type: Handshake
        record.extend_from_slice(&[0x03, 0x01]); // version: TLS 1.0 (for compat)
        record.extend_from_slice(&(handshake.len() as u16).to_be_bytes());
        record.extend_from_slice(&handshake);

        record
    }

    /// Wrap data as a TLS Application Data record.
    fn pack_tls_app_data(&self, data: &[u8]) -> Vec<u8> {
        let mut record = Vec::with_capacity(5 + data.len());
        record.push(0x17); // content type: Application Data
        record.extend_from_slice(&[0x03, 0x03]); // version: TLS 1.2
        record.extend_from_slice(&(data.len() as u16).to_be_bytes());
        record.extend_from_slice(data);
        record
    }

    /// Try to parse the TLS server response. Skip handshake records, return
    /// Application Data payload.
    fn try_parse_server_response(&mut self) -> Result<Vec<u8>> {
        let mut result = Vec::new();
        let mut offset = 0;

        while offset + 5 <= self.recv_buf.len() {
            let content_type = self.recv_buf[offset];
            let record_len = u16::from_be_bytes([self.recv_buf[offset + 3], self.recv_buf[offset + 4]]) as usize;

            if offset + 5 + record_len > self.recv_buf.len() {
                break; // incomplete record
            }

            if content_type == 0x17 {
                // Application Data — this is our payload.
                result.extend_from_slice(&self.recv_buf[offset + 5..offset + 5 + record_len]);
                self.has_recv_header = true;
            }
            // Skip handshake (0x16) and change cipher spec (0x14) records.
            offset += 5 + record_len;
        }

        // Consume processed bytes.
        if offset > 0 {
            self.recv_buf.drain(..offset);
        }
        Ok(result)
    }

    /// Unwrap TLS Application Data records.
    fn unpack_tls_records(&mut self, data: &[u8]) -> Result<Vec<u8>> {
        self.recv_buf.extend_from_slice(data);
        let mut result = Vec::new();
        let mut offset = 0;

        while offset + 5 <= self.recv_buf.len() {
            let content_type = self.recv_buf[offset];
            let record_len = u16::from_be_bytes([self.recv_buf[offset + 3], self.recv_buf[offset + 4]]) as usize;

            if offset + 5 + record_len > self.recv_buf.len() {
                break;
            }

            if content_type == 0x17 {
                result.extend_from_slice(&self.recv_buf[offset + 5..offset + 5 + record_len]);
            }
            offset += 5 + record_len;
        }

        if offset > 0 {
            self.recv_buf.drain(..offset);
        }
        Ok(result)
    }
}
