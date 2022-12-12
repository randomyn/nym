// Copyright 2022 - Nym Technologies SA <contact@nymtech.net>
// SPDX-License-Identifier: Apache-2.0

use crate::client::cover_traffic_stream::LoopCoverTrafficStream;
use crate::client::inbound_messages::{InputMessage, InputMessageReceiver, InputMessageSender};
use crate::client::key_manager::KeyManager;
use crate::client::mix_traffic::{BatchMixMessageSender, MixTrafficController};
use crate::client::real_messages_control;
use crate::client::real_messages_control::RealMessagesController;
use crate::client::received_buffer::{
    ReceivedBufferRequestReceiver, ReceivedBufferRequestSender, ReceivedMessagesBufferController,
};
use crate::client::replies::reply_controller;
use crate::client::replies::reply_controller::{ReplyControllerReceiver, ReplyControllerSender};
use crate::client::replies::reply_storage::{
    CombinedReplyStorage, PersistentReplyStorage, ReplyStorageBackend, SentReplyKeys,
};
use crate::client::topology_control::{
    TopologyAccessor, TopologyRefresher, TopologyRefresherConfig,
};
use crate::config::{Config, DebugConfig, GatewayEndpointConfig};
use crate::error::ClientCoreError;
use crate::spawn_future;
use client_connections::{ConnectionCommandReceiver, ConnectionCommandSender, LaneQueueLengths};
use crypto::asymmetric::{encryption, identity};
use futures::channel::mpsc;
use gateway_client::bandwidth::BandwidthController;
use gateway_client::{
    AcknowledgementReceiver, AcknowledgementSender, GatewayClient, MixnetMessageReceiver,
    MixnetMessageSender,
};
use log::info;
use nymsphinx::acknowledgements::AckKey;
use nymsphinx::addressing::clients::Recipient;
use nymsphinx::addressing::nodes::NodeIdentity;
use std::sync::Arc;
use std::time::Duration;
use task::{ShutdownListener, ShutdownNotifier};
use url::Url;

#[cfg(all(not(target_arch = "wasm32"), feature = "fs-surb-storage"))]
pub mod non_wasm_helpers;

pub struct ClientInput {
    pub shared_lane_queue_lengths: LaneQueueLengths,
    pub connection_command_sender: ConnectionCommandSender,
    pub input_sender: InputMessageSender,
}

pub struct ClientOutput {
    pub received_buffer_request_sender: ReceivedBufferRequestSender,
}

pub enum ClientInputStatus {
    AwaitingProducer { client_input: ClientInput },
    Connected,
}

impl ClientInputStatus {
    pub fn register_producer(&mut self) -> ClientInput {
        match std::mem::replace(self, ClientInputStatus::Connected) {
            ClientInputStatus::AwaitingProducer { client_input } => client_input,
            ClientInputStatus::Connected => panic!("producer was already registered before"),
        }
    }
}

pub enum ClientOutputStatus {
    AwaitingConsumer { client_output: ClientOutput },
    Connected,
}

impl ClientOutputStatus {
    pub fn register_consumer(&mut self) -> ClientOutput {
        match std::mem::replace(self, ClientOutputStatus::Connected) {
            ClientOutputStatus::AwaitingConsumer { client_output } => client_output,
            ClientOutputStatus::Connected => panic!("consumer was already registered before"),
        }
    }
}

pub struct BaseClientBuilder<'a, B> {
    // due to wasm limitations I had to split it like this : (
    gateway_config: &'a GatewayEndpointConfig,
    debug_config: &'a DebugConfig,
    disabled_credentials: bool,
    validator_api_endpoints: Vec<Url>,
    reply_storage_backend: B,

    bandwidth_controller: Option<BandwidthController>,
    key_manager: KeyManager,
}

impl<'a, B> BaseClientBuilder<'a, B>
where
    B: ReplyStorageBackend + Send + Sync + 'static,
{
    pub fn new_from_base_config<T>(
        base_config: &'a Config<T>,
        key_manager: KeyManager,
        bandwidth_controller: Option<BandwidthController>,
        reply_storage_backend: B,
    ) -> BaseClientBuilder<'a, B> {
        BaseClientBuilder {
            gateway_config: base_config.get_gateway_endpoint_config(),
            debug_config: base_config.get_debug_config(),
            disabled_credentials: base_config.get_disabled_credentials_mode(),
            validator_api_endpoints: base_config.get_validator_api_endpoints(),
            bandwidth_controller,
            reply_storage_backend,
            key_manager,
        }
    }

    pub fn new(
        gateway_config: &'a GatewayEndpointConfig,
        debug_config: &'a DebugConfig,
        key_manager: KeyManager,
        bandwidth_controller: Option<BandwidthController>,
        reply_storage_backend: B,
        disabled_credentials: bool,
        validator_api_endpoints: Vec<Url>,
    ) -> BaseClientBuilder<'a, B> {
        BaseClientBuilder {
            gateway_config,
            debug_config,
            disabled_credentials,
            validator_api_endpoints,
            bandwidth_controller,
            reply_storage_backend,
            key_manager,
        }
    }

    pub fn as_mix_recipient(&self) -> Recipient {
        Recipient::new(
            *self.key_manager.identity_keypair().public_key(),
            *self.key_manager.encryption_keypair().public_key(),
            // TODO: below only works under assumption that gateway address == gateway id
            // (which currently is true)
            NodeIdentity::from_base58_string(&self.gateway_config.gateway_id).unwrap(),
        )
    }

    // future constantly pumping loop cover traffic at some specified average rate
    // the pumped traffic goes to the MixTrafficController
    fn start_cover_traffic_stream(
        debug_config: &DebugConfig,
        ack_key: Arc<AckKey>,
        self_address: Recipient,
        topology_accessor: TopologyAccessor,
        mix_tx: BatchMixMessageSender,
        shutdown: ShutdownListener,
    ) {
        info!("Starting loop cover traffic stream...");

        let mut stream = LoopCoverTrafficStream::new(
            ack_key,
            debug_config.average_ack_delay,
            debug_config.average_packet_delay,
            debug_config.loop_cover_traffic_average_delay,
            mix_tx,
            self_address,
            topology_accessor,
        );

        if let Some(size) = debug_config.use_extended_packet_size {
            log::debug!("Setting extended packet size: {:?}", size);
            stream.set_custom_packet_size(size.into());
        }

        stream.start_with_shutdown(shutdown);
    }

    #[allow(clippy::too_many_arguments)]
    fn start_real_traffic_controller(
        controller_config: real_messages_control::Config,
        topology_accessor: TopologyAccessor,
        ack_receiver: AcknowledgementReceiver,
        input_receiver: InputMessageReceiver,
        mix_sender: BatchMixMessageSender,
        reply_storage: CombinedReplyStorage,
        reply_controller_sender: ReplyControllerSender,
        reply_controller_receiver: ReplyControllerReceiver,
        lane_queue_lengths: LaneQueueLengths,
        client_connection_rx: ConnectionCommandReceiver,
        shutdown: ShutdownListener,
    ) {
        info!("Starting real traffic stream...");

        RealMessagesController::new(
            controller_config,
            ack_receiver,
            input_receiver,
            mix_sender,
            topology_accessor,
            reply_storage,
            reply_controller_sender,
            reply_controller_receiver,
            lane_queue_lengths,
            client_connection_rx,
        )
        .start_with_shutdown(shutdown);
    }

    // buffer controlling all messages fetched from provider
    // required so that other components would be able to use them (say the websocket)
    fn start_received_messages_buffer_controller(
        local_encryption_keypair: Arc<encryption::KeyPair>,
        query_receiver: ReceivedBufferRequestReceiver,
        mixnet_receiver: MixnetMessageReceiver,
        reply_key_storage: SentReplyKeys,
        reply_controller_sender: ReplyControllerSender,
        shutdown: ShutdownListener,
    ) {
        info!("Starting received messages buffer controller...");
        ReceivedMessagesBufferController::new(
            local_encryption_keypair,
            query_receiver,
            mixnet_receiver,
            reply_key_storage,
            reply_controller_sender,
        )
        .start_with_shutdown(shutdown)
    }

    async fn start_gateway_client(
        &mut self,
        mixnet_message_sender: MixnetMessageSender,
        ack_sender: AcknowledgementSender,
        shutdown: ShutdownListener,
    ) -> GatewayClient {
        let gateway_id = self.gateway_config.gateway_id.clone();
        if gateway_id.is_empty() {
            panic!("The identity of the gateway is unknown - did you run `nym-client` init?")
        }
        let gateway_owner = self.gateway_config.gateway_owner.clone();
        if gateway_owner.is_empty() {
            panic!("The owner of the gateway is unknown - did you run `nym-client` init?")
        }
        let gateway_address = self.gateway_config.gateway_listener.clone();
        if gateway_address.is_empty() {
            panic!("The address of the gateway is unknown - did you run `nym-client` init?")
        }

        let gateway_identity = identity::PublicKey::from_base58_string(gateway_id)
            .expect("provided gateway id is invalid!");

        // disgusting wasm workaround since there's no key persistence there (nor `client init`)
        let shared_key = if self.key_manager.gateway_key_set() {
            Some(self.key_manager.gateway_shared_key())
        } else {
            None
        };

        let mut gateway_client = GatewayClient::new(
            gateway_address,
            self.key_manager.identity_keypair(),
            gateway_identity,
            gateway_owner,
            shared_key,
            mixnet_message_sender,
            ack_sender,
            self.debug_config.gateway_response_timeout,
            self.bandwidth_controller.take(),
            shutdown,
        );

        gateway_client.set_disabled_credentials_mode(self.disabled_credentials);

        gateway_client
            .authenticate_and_start()
            .await
            .expect("could not authenticate and start up the gateway connection");

        gateway_client
    }

    // future responsible for periodically polling directory server and updating
    // the current global view of topology
    async fn start_topology_refresher(
        validator_api_urls: Vec<Url>,
        refresh_rate: Duration,
        topology_accessor: TopologyAccessor,
        shutdown: ShutdownListener,
    ) -> Result<(), ClientCoreError<B>> {
        let topology_refresher_config = TopologyRefresherConfig::new(
            validator_api_urls,
            refresh_rate,
            env!("CARGO_PKG_VERSION").to_string(),
        );
        let mut topology_refresher =
            TopologyRefresher::new(topology_refresher_config, topology_accessor);
        // before returning, block entire runtime to refresh the current network view so that any
        // components depending on topology would see a non-empty view
        info!("Obtaining initial network topology");
        topology_refresher.refresh().await;

        if let Err(err) = topology_refresher.ensure_topology_is_routable().await {
            log::error!(
                "The current network topology seem to be insufficient to route any packets through \
                - check if enough nodes and a gateway are online - source: {err}"
            );
            return Err(ClientCoreError::InsufficientNetworkTopology(err));
        }

        info!("Starting topology refresher...");
        topology_refresher.start_with_shutdown(shutdown);
        Ok(())
    }

    // controller for sending sphinx packets to mixnet (either real traffic or cover traffic)
    // TODO: if we want to send control messages to gateway_client, this CAN'T take the ownership
    // over it. Perhaps GatewayClient needs to be thread-shareable or have some channel for
    // requests?
    fn start_mix_traffic_controller(
        gateway_client: GatewayClient,
        shutdown: ShutdownListener,
    ) -> BatchMixMessageSender {
        info!("Starting mix traffic controller...");
        let (mix_traffic_controller, mix_tx) = MixTrafficController::new(gateway_client);
        mix_traffic_controller.start_with_shutdown(shutdown);
        mix_tx
    }

    async fn setup_persistent_reply_storage(
        backend: B,
        shutdown: ShutdownListener,
    ) -> Result<CombinedReplyStorage, ClientCoreError<B>> {
        let persistent_storage = PersistentReplyStorage::new(backend);
        let mem_store = persistent_storage
            .load_state_from_backend()
            .await
            .map_err(|err| ClientCoreError::SurbStorageError { source: err })?;

        let store_clone = mem_store.clone();
        spawn_future(async move {
            persistent_storage
                .flush_on_shutdown(store_clone, shutdown)
                .await
        });

        Ok(mem_store)
    }

    pub async fn start_base(mut self) -> Result<BaseClient, ClientCoreError<B>> {
        info!("Starting nym client");
        // channels for inter-component communication
        // TODO: make the channels be internally created by the relevant components
        // rather than creating them here, so say for example the buffer controller would create the request channels
        // and would allow anyone to clone the sender channel

        // unwrapped_sphinx_sender is the transmitter of mixnet messages received from the gateway
        // unwrapped_sphinx_receiver is the receiver for said messages - used by ReceivedMessagesBuffer
        let (mixnet_messages_sender, mixnet_messages_receiver) = mpsc::unbounded();

        // used for announcing connection or disconnection of a channel for pushing re-assembled messages to
        let (received_buffer_request_sender, received_buffer_request_receiver) = mpsc::unbounded();

        // channels responsible for controlling real messages
        let (input_sender, input_receiver) = tokio::sync::mpsc::channel::<InputMessage>(1);

        // channels responsible for controlling ack messages
        let (ack_sender, ack_receiver) = mpsc::unbounded();
        let shared_topology_accessor = TopologyAccessor::new();

        // Shutdown notifier for signalling tasks to stop
        let shutdown = ShutdownNotifier::default();

        // channels responsible for dealing with reply-related fun
        let (reply_controller_sender, reply_controller_receiver) =
            reply_controller::new_control_channels();

        let self_address = self.as_mix_recipient();

        // the components are started in very specific order. Unless you know what you are doing,
        // do not change that.
        let gateway_client = self
            .start_gateway_client(mixnet_messages_sender, ack_sender, shutdown.subscribe())
            .await;

        let reply_storage =
            Self::setup_persistent_reply_storage(self.reply_storage_backend, shutdown.subscribe())
                .await?;

        Self::start_topology_refresher(
            self.validator_api_endpoints.clone(),
            self.debug_config.topology_refresh_rate,
            shared_topology_accessor.clone(),
            shutdown.subscribe(),
        )
        .await?;

        Self::start_received_messages_buffer_controller(
            self.key_manager.encryption_keypair(),
            received_buffer_request_receiver,
            mixnet_messages_receiver,
            reply_storage.key_storage(),
            reply_controller_sender.clone(),
            shutdown.subscribe(),
        );

        // The sphinx_message_sender is the transmitter for any component generating sphinx packets
        // that are to be sent to the mixnet. They are used by cover traffic stream and real
        // traffic stream.
        // The MixTrafficController then sends the actual traffic
        let sphinx_message_sender =
            Self::start_mix_traffic_controller(gateway_client, shutdown.subscribe());

        // Channels that the websocket listener can use to signal downstream to the real traffic
        // controller that connections are closed.
        let (client_connection_tx, client_connection_rx) = mpsc::unbounded();

        // Shared queue length data. Published by the `OutQueueController` in the client, and used
        // primarily to throttle incoming connections (e.g socks5 for attached network-requesters)
        let shared_lane_queue_lengths = LaneQueueLengths::new();

        let mut controller_config = real_messages_control::Config::new(
            self.debug_config,
            self.key_manager.ack_key(),
            self_address,
        );

        if let Some(size) = self.debug_config.use_extended_packet_size {
            log::debug!("Setting extended packet size: {:?}", size);
            controller_config.set_custom_packet_size(size.into());
        }

        Self::start_real_traffic_controller(
            controller_config,
            shared_topology_accessor.clone(),
            ack_receiver,
            input_receiver,
            sphinx_message_sender.clone(),
            reply_storage,
            reply_controller_sender,
            reply_controller_receiver,
            shared_lane_queue_lengths.clone(),
            client_connection_rx,
            shutdown.subscribe(),
        );

        if !self.debug_config.disable_loop_cover_traffic_stream {
            Self::start_cover_traffic_stream(
                self.debug_config,
                self.key_manager.ack_key(),
                self_address,
                shared_topology_accessor,
                sphinx_message_sender,
                shutdown.subscribe(),
            );
        }

        info!("Client startup finished!");
        info!("The address of this client is: {self_address}");

        Ok(BaseClient {
            client_input: ClientInputStatus::AwaitingProducer {
                client_input: ClientInput {
                    shared_lane_queue_lengths,
                    connection_command_sender: client_connection_tx,
                    input_sender,
                },
            },
            client_output: ClientOutputStatus::AwaitingConsumer {
                client_output: ClientOutput {
                    received_buffer_request_sender,
                },
            },
            shutdown_notifier: shutdown,
        })
    }
}

pub struct BaseClient {
    pub client_input: ClientInputStatus,
    pub client_output: ClientOutputStatus,

    pub shutdown_notifier: ShutdownNotifier,
}
