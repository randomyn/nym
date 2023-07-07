// Copyright 2021 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use log::*;
use nym_sphinx::addressing::nodes::NymNodeRoutingAddress;
use nym_sphinx::framing::codec::NymCodec;
use nym_sphinx::framing::packet::FramedNymPacket;
use nym_sphinx::params::PacketType;
use nym_sphinx::NymPacket;
use quinn::{ClientConfig, Endpoint};
use std::collections::HashMap;
use std::io;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::time::sleep;
use tokio_util::codec::FramedWrite;

pub struct Config {
    initial_reconnection_backoff: Duration,
    maximum_reconnection_backoff: Duration,
    initial_connection_timeout: Duration,
    maximum_connection_buffer_size: usize,
    use_legacy_version: bool,
}

impl Config {
    pub fn new(
        initial_reconnection_backoff: Duration,
        maximum_reconnection_backoff: Duration,
        initial_connection_timeout: Duration,
        maximum_connection_buffer_size: usize,
        use_legacy_version: bool,
    ) -> Self {
        Config {
            initial_reconnection_backoff,
            maximum_reconnection_backoff,
            initial_connection_timeout,
            maximum_connection_buffer_size,
            use_legacy_version,
        }
    }
}

pub trait SendWithoutResponse {
    // Without response in this context means we will not listen for anything we might get back (not
    // that we should get anything), including any possible io errors
    fn send_without_response(
        &mut self,
        address: NymNodeRoutingAddress,
        packet: NymPacket,
        packet_type: PacketType,
    ) -> io::Result<()>;
}

pub struct Client {
    conn_new: HashMap<NymNodeRoutingAddress, ConnectionSender>,
    config: Config,
}

struct ConnectionSender {
    channel: mpsc::Sender<FramedNymPacket>,
    current_reconnection_attempt: Arc<AtomicU32>,
    last_used: Instant,
}

impl ConnectionSender {
    fn new(channel: mpsc::Sender<FramedNymPacket>) -> Self {
        ConnectionSender {
            channel,
            current_reconnection_attempt: Arc::new(AtomicU32::new(0)),
            last_used: Instant::now(),
        }
    }
}

impl Client {
    pub fn new(config: Config) -> Client {
        Client {
            conn_new: HashMap::new(),
            config,
        }
    }

    async fn manage_connection(
        address: SocketAddr,
        mut receiver: mpsc::Receiver<FramedNymPacket>,
        connection_timeout: Duration,
        current_reconnection: &AtomicU32,
    ) {
        let mut endpoint = Endpoint::client("0.0.0.0:0".parse::<SocketAddr>().unwrap()).unwrap();
        endpoint.set_default_client_config(configure_client());

        let conn = match tokio::time::timeout(
            connection_timeout,
            endpoint.connect(address, "mixnode").unwrap(),
        )
        .await
        {
            Ok(stream_res) => match stream_res {
                Ok(connection) => {
                    debug!("Managed to establish connection to {}", address);
                    // if we managed to connect, reset the reconnection count (whatever it might have been)
                    current_reconnection.store(0, Ordering::Release);
                    connection
                }
                Err(err) => {
                    debug!(
                        "failed to establish connection to {} (err: {})",
                        address, err
                    );
                    return;
                }
            },
            Err(_) => {
                debug!(
                    "failed to connect to {} within {:?}",
                    address, connection_timeout
                );

                // we failed to connect - increase reconnection attempt
                current_reconnection.fetch_add(1, Ordering::SeqCst);
                return;
            }
        };
        loop {
            let pkt = match receiver.next().await {
                Some(pkt) => pkt,
                None => {
                    debug!("No more packet to send to {}", address);
                    return;
                }
            };
            let send = match conn.open_uni().await {
                Ok(send_stream) => send_stream,
                Err(err) => {
                    error!("Failed to open uni stream, dropping packet - {err:?}");
                    return; //We shouldn't get a time out here, it should be handled higher
                }
            };
            let mut framed_stream = FramedWrite::new(send, NymCodec);
            if let Err(err) = framed_stream.send(pkt).await {
                warn!("Failed to forward packets to {} - {err}", address);
            }
        }
    }

    /// If we're trying to reconnect, determine how long we should wait.
    fn determine_backoff(&self, current_attempt: u32) -> Option<Duration> {
        if current_attempt == 0 {
            None
        } else {
            let exp = 2_u32.checked_pow(current_attempt);
            let backoff = exp
                .and_then(|exp| self.config.initial_reconnection_backoff.checked_mul(exp))
                .unwrap_or(self.config.maximum_reconnection_backoff);

            Some(std::cmp::min(
                backoff,
                self.config.maximum_reconnection_backoff,
            ))
        }
    }

    fn make_connection(&mut self, address: NymNodeRoutingAddress, pending_packet: FramedNymPacket) {
        let (mut sender, receiver) = mpsc::channel(self.config.maximum_connection_buffer_size);

        // this CAN'T fail because we just created the channel which has a non-zero capacity
        if self.config.maximum_connection_buffer_size > 0 {
            sender.try_send(pending_packet).unwrap();
        }

        // if we already tried to connect to `address` before, grab the current attempt count
        let current_reconnection_attempt = if let Some(existing) = self.conn_new.get_mut(&address) {
            existing.channel = sender;
            Arc::clone(&existing.current_reconnection_attempt)
        } else {
            let new_entry = ConnectionSender::new(sender);
            let current_attempt = Arc::clone(&new_entry.current_reconnection_attempt);
            self.conn_new.insert(address, new_entry);
            current_attempt
        };

        // load the actual value.
        let reconnection_attempt = current_reconnection_attempt.load(Ordering::Acquire);
        let backoff = self.determine_backoff(reconnection_attempt);

        // copy the value before moving into another task
        let initial_connection_timeout = self.config.initial_connection_timeout;

        tokio::spawn(async move {
            // before executing the manager, wait for what was specified, if anything
            if let Some(backoff) = backoff {
                trace!("waiting for {:?} before attempting connection", backoff);
                sleep(backoff).await;
            }

            Self::manage_connection(
                address.into(),
                receiver,
                initial_connection_timeout,
                &current_reconnection_attempt,
            )
            .await
        });
    }
}

impl SendWithoutResponse for Client {
    fn send_without_response(
        &mut self,
        address: NymNodeRoutingAddress,
        packet: NymPacket,
        packet_type: PacketType,
    ) -> io::Result<()> {
        trace!("Sending packet to {:?}", address);
        let framed_packet =
            FramedNymPacket::new(packet, packet_type, self.config.use_legacy_version);

        if let Some(sender) = self.conn_new.get_mut(&address) {
            if sender.last_used.elapsed().as_millis() > 9_000 {
                // default timeout is 10sec, let's take some margin for the operation to run.
                //connection is near timeout, let's just recreate one
                sender.channel.close_channel();
                self.conn_new.remove(&address);
                debug!(
                    "connection near or past timemout. Reconnecting to {}",
                    address
                );
                self.make_connection(address, framed_packet);
                Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "re-connection is in progress",
                ))
            } else {
                sender.last_used = Instant::now();
                if let Err(err) = sender.channel.try_send(framed_packet) {
                    if err.is_full() {
                        debug!("Connection to {} seems to not be able to handle all the traffic - dropping the current packet", address);
                        // it's not a 'big' error, but we did not manage to send the packet
                        // if the queue is full, we can't really do anything but to drop the packet
                        Err(io::Error::new(
                            io::ErrorKind::WouldBlock,
                            "connection queue is full",
                        ))
                    } else if err.is_disconnected() {
                        debug!(
                            "Connection to {} seems to be dead. attempting to re-establish it...",
                            address
                        );
                        // it's not a 'big' error, but we did not manage to send the packet, but queue
                        // it up to send it as soon as the connection is re-established
                        self.make_connection(address, err.into_inner());
                        Err(io::Error::new(
                            io::ErrorKind::ConnectionAborted,
                            "reconnection attempt is in progress",
                        ))
                    } else {
                        // this can't really happen, but let's safe-guard against it in case something changes in futures library
                        Err(io::Error::new(
                            io::ErrorKind::Other,
                            "unknown connection buffer error",
                        ))
                    }
                } else {
                    Ok(())
                }
            }
        } else {
            // there was never a connection to begin with
            debug!("establishing initial connection to {}", address);
            // it's not a 'big' error, but we did not manage to send the packet, but queue the packet
            // for sending for as soon as the connection is created
            self.make_connection(address, framed_packet);
            Err(io::Error::new(
                io::ErrorKind::NotConnected,
                "connection is in progress",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_client() -> Client {
        Client::new(Config {
            initial_reconnection_backoff: Duration::from_millis(10_000),
            maximum_reconnection_backoff: Duration::from_millis(300_000),
            initial_connection_timeout: Duration::from_millis(1_500),
            maximum_connection_buffer_size: 128,
            use_legacy_version: false,
        })
    }

    #[test]
    fn determining_backoff_works_regardless_of_attempt() {
        let client = dummy_client();
        assert!(client.determine_backoff(0).is_none());
        assert!(client.determine_backoff(1).is_some());
        assert!(client.determine_backoff(2).is_some());
        assert_eq!(
            client.determine_backoff(16).unwrap(),
            client.config.maximum_reconnection_backoff
        );
        assert_eq!(
            client.determine_backoff(32).unwrap(),
            client.config.maximum_reconnection_backoff
        );
        assert_eq!(
            client.determine_backoff(1024).unwrap(),
            client.config.maximum_reconnection_backoff
        );
        assert_eq!(
            client.determine_backoff(65536).unwrap(),
            client.config.maximum_reconnection_backoff
        );
        assert_eq!(
            client.determine_backoff(u32::MAX).unwrap(),
            client.config.maximum_reconnection_backoff
        );
    }
}

// Implementation of `ServerCertVerifier` that verifies everything as trustworthy.
struct SkipServerVerification;

impl SkipServerVerification {
    fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[rustls::Certificate],
        _server_name: &rustls::ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: std::time::SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}

fn configure_client() -> ClientConfig {
    let crypto = rustls::ClientConfig::builder()
        .with_safe_defaults()
        .with_custom_certificate_verifier(SkipServerVerification::new())
        .with_no_client_auth();

    ClientConfig::new(Arc::new(crypto))
}
