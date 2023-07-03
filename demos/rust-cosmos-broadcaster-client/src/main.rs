use clap::{CommandFactory, Parser, Subcommand, Args};
// use cosmrs::bip32::secp256k1::elliptic_curve::generic_array::sequence;
// use nym_validator_client::nyxd::CosmWasmClient;
// use nym_validator_client::signing::direct_wallet::DirectSecp256k1HdWallet;
// use nym_validator_client::signing::tx_signer::TxSigner;
// use nym_validator_client::signing::SignerData;
// use cosmrs::bank::MsgSend;
// use cosmrs::rpc::{self, HttpClient};
// use cosmrs::tx::Msg;
// use cosmrs::{tx, AccountId, Coin, Denom};
// use bip39; 
mod commands; 

#[derive(Debug, Parser)]
#[clap(name = "nym cosmos tx signer ")]
#[clap(about = "binary with which users can perform offline signing and transmission of signed tx to broadcaster via the mixnet ")]
struct Cli {
    #[clap(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
enum Commands {
   /// sign a transaction offline  
    OfflineSignTx(OfflineSignTx), 
    /// send signed tx to SP for broadcast 
    SendTx(SendTx)
}

#[derive(Debug, Clone, Args)]
struct OfflineSignTx {
    /// mnemonic of signing + sending account (you!)
    mnemonic: String, // TODO input validation.. look @ file loading first 
    /// recipient nyx chain address
    to: String // TODO switch to proper cosmos address type  
}

#[derive(Debug, Args)]
struct SendTx {
    /// the address of the nym service to send yr signed tx 
    sp_address: String // TODO replace with mixnet address type  
}

#[tokio::main]
async fn main() {
    
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::OfflineSignTx(OfflineSignTx { mnemonic, to } )) => {
            let tx_bytes = commands::commands::offline_sign(mnemonic, to).await;         
            
            // TODO save as global var to pass to sendtx() 
            println!("{:?}", tx_bytes.iter().collect::<Vec<_>>()); 
            println!("signed"); 
        }
        Some(Commands::SendTx(sp_address)) => {
            todo!(); 
        }       
        None => {}
    }
}