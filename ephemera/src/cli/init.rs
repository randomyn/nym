use clap::{Args, Parser};
use serde::{Deserialize, Serialize};

use crate::config::{
    BlockManagerConfiguration, Configuration, DatabaseConfiguration, HttpConfiguration,
    Libp2pConfiguration, MembershipKind as ConfigMembershipKind, NodeConfiguration,
    WebsocketConfiguration,
};
use crate::crypto::{EphemeraKeypair, Keypair};

//network settings
const DEFAULT_LISTEN_ADDRESS: &str = "127.0.0.1";
const DEFAULT_PROT_LISTEN_PORT: u16 = 3000;
const DEFAULT_WS_LISTEN_PORT: u16 = 3001;
const DEFAULT_HTTP_LISTEN_PORT: u16 = 3002;

//libp2p settings
const DEFAULT_MESSAGES_TOPIC_NAME: &str = "nym-ephemera-proposed";
const DEFAULT_HEARTBEAT_INTERVAL_SEC: u64 = 1;

#[derive(Args, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
#[group(required = true, multiple = false)]
pub struct MembershipKind {
    /// Requires the threshold of peers returned by membership provider to be online
    #[clap(long)]
    threshold: Option<u64>,
    /// Requires that all peers returned by membership provider peers to be online
    #[clap(long)]
    all: bool,
    /// Membership is just all online peers from the set returned by membership provider
    #[clap(long)]
    any: bool,
}

impl Default for MembershipKind {
    fn default() -> Self {
        MembershipKind {
            threshold: None,
            all: false,
            any: true,
        }
    }
}

impl From<MembershipKind> for ConfigMembershipKind {
    fn from(kind: MembershipKind) -> Self {
        match (kind.threshold, kind.all, kind.any) {
            //FIXME: use threshold value
            (Some(_), false, false) => ConfigMembershipKind::Threshold,
            (None, true, false) => ConfigMembershipKind::AllOnline,
            (None, false, true) => ConfigMembershipKind::AnyOnline,
            _ => panic!("Invalid membership kind"),
        }
    }
}

#[derive(Parser, Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Cmd {
    #[clap(long, default_value = DEFAULT_LISTEN_ADDRESS)]
    /// The IP address to listen on
    pub ip: String,
    /// The port which Ephemera uses for peer to peer communication
    #[clap(long, default_value_t = DEFAULT_PROT_LISTEN_PORT)]
    pub protocol_port: u16,
    /// The port which Ephemera listens on for websocket subscriptions
    #[clap(long, default_value_t = DEFAULT_WS_LISTEN_PORT)]
    pub websocket_port: u16,
    /// The port which Ephemera listens on for http api
    #[clap(long, default_value_t = DEFAULT_HTTP_LISTEN_PORT)]
    pub http_api_port: u16,
    /// Either this node produces blocks or not
    #[clap(long, default_value_t = true)]
    pub block_producer: bool,
    /// At which interval to produce blocks
    #[clap(long, default_value_t = 30)]
    pub block_creation_interval_sec: u64,
    /// When next block is created before previous one is finished, should we repeat it with the same messages
    #[clap(long, default_value_t = false)]
    pub repeat_last_block_messages: bool,
    /// The interval at which Ephemera requests the list of members
    #[clap(long, default_value_t = 60 * 60)]
    pub members_provider_delay_sec: u64,
    /// A rule how to choose members based on their online status
    #[command(flatten)]
    pub membership_kind: MembershipKind,
}

impl Default for Cmd {
    fn default() -> Self {
        Cmd {
            ip: String::from(DEFAULT_LISTEN_ADDRESS),
            protocol_port: DEFAULT_PROT_LISTEN_PORT,
            websocket_port: DEFAULT_WS_LISTEN_PORT,
            http_api_port: DEFAULT_HTTP_LISTEN_PORT,
            block_producer: true,
            block_creation_interval_sec: 30,
            repeat_last_block_messages: false,
            members_provider_delay_sec: 60 * 60,
            membership_kind: MembershipKind::default(),
        }
    }
}

impl Cmd {
    /// # Panics
    /// Panics if the config file already exists.
    pub fn execute(self, id: Option<&str>) {
        assert!(
            Configuration::try_load_from_home_dir().is_err(),
            "Configuration file already exists",
        );

        let path = Configuration::ephemera_root_dir(id).unwrap();
        println!("Creating ephemera node configuration in: {path:?}",);

        let db_dir = path.join("db");
        let rocksdb_path = db_dir.join("rocksdb");
        let sqlite_path = db_dir.join("ephemera.sqlite");
        std::fs::create_dir_all(&rocksdb_path).unwrap();
        std::fs::File::create(&sqlite_path).unwrap();

        let keypair = Keypair::generate(None);
        let private_key = keypair.to_base58();

        let configuration = Configuration {
            node: NodeConfiguration {
                ip: self.ip,
                private_key,
            },
            libp2p: Libp2pConfiguration {
                port: self.protocol_port,
                ephemera_msg_topic_name: DEFAULT_MESSAGES_TOPIC_NAME.to_string(),
                heartbeat_interval_sec: DEFAULT_HEARTBEAT_INTERVAL_SEC,
                members_provider_delay_sec: self.members_provider_delay_sec,
                membership_kind: self.membership_kind.into(),
            },
            storage: DatabaseConfiguration {
                rocksdb_path: rocksdb_path.as_os_str().to_str().unwrap().to_string(),
                sqlite_path: sqlite_path.as_os_str().to_str().unwrap().to_string(),
                create_if_not_exists: true,
            },
            websocket: WebsocketConfiguration {
                port: self.websocket_port,
            },
            http: HttpConfiguration {
                port: self.http_api_port,
            },
            block_manager: BlockManagerConfiguration {
                producer: self.block_producer,
                creation_interval_sec: self.block_creation_interval_sec,
                repeat_last_block_messages: self.repeat_last_block_messages,
            },
        };

        if let Err(err) = configuration.try_write_home_dir(id) {
            eprintln!("Error creating configuration file: {err:?}",);
        }
    }
}
