use bitcoin::util::amount::Amount;
use bitcoin::Network;
use bitcoin_wallet::mnemonic;
use bitcoincore_rpc::{Client, RpcApi};

use teleport::fidelity_bonds::YearAndMonth;
use teleport::maker_protocol::MakerBehavior;
use teleport::settings::Settings;
use teleport::wallet_sync::{Wallet, WalletSyncAddressAmount};

use tempfile::tempdir;

use serde_json::Value;

use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::{thread, time};

static WATCHTOWER_DATA: &str = "watchtower.dat";
static TAKER: &str = "taker-wallet";
static MAKER1: &str = "maker-wallet-1";
static MAKER2: &str = "maker-wallet-2";

// Helper function to create new wallet
fn create_wallet_and_import(rpc: &Client, filename: PathBuf) -> Wallet {
    let mnemonic =
        mnemonic::Mnemonic::new_random(bitcoin_wallet::account::MasterKeyEntropy::Sufficient)
            .unwrap();

    Wallet::save_new_wallet_file(&filename, mnemonic.to_string(), "".to_string()).unwrap();

    let wallet =
        Wallet::load_wallet_from_file(filename, Network::Regtest, WalletSyncAddressAmount::Testing)
            .unwrap();
    // import intital addresses to core
    wallet
        .import_initial_addresses(
            rpc,
            &wallet
                .get_hd_wallet_descriptors(rpc)
                .unwrap()
                .iter()
                .collect::<Vec<&String>>(),
            &Vec::<_>::new(),
            &Vec::<_>::new(),
        )
        .unwrap();

    wallet
}

pub fn generate_1_block(rpc: &Client) {
    rpc.generate_to_address(1, &rpc.get_new_address(None, None).unwrap())
        .unwrap();
}

// This test requires a bitcoin regtest node running in local machine with a
// wallet name `teleport` loaded and have enough balance to execute transactions.
#[tokio::test]
async fn test_standard_coinswap() {
    let test_dir = tempdir().expect("Error making temporary directory");
    let test_path = test_dir.path().to_owned();
    Settings::init_settings(&test_path);
    teleport::setup_teleport();

    // TODO: This only works if the RPC cookie file exists in its default location
    // Need to figure out a good solution for setting test credentials and other test config
    let (rpc, network) = teleport::get_bitcoin_rpc().unwrap();
    assert_eq!(network, Network::Regtest);

    // unlock all utxos to avoid "insufficient fund" error
    rpc.call::<Value>("lockunspent", &[Value::Bool(true)])
        .unwrap();

    // create taker wallet
    let mut taker_wallet = create_wallet_and_import(&rpc, TAKER.into());

    // create maker1 wallet
    let mut maker1_wallet = create_wallet_and_import(&rpc, MAKER1.into());

    // create maker2 wallet
    let mut maker2_wallet = create_wallet_and_import(&rpc, MAKER2.into());

    // Check files are created
    let wallet_path = test_path.join("wallets");
    assert!(wallet_path.join(TAKER).exists());
    assert!(wallet_path.join(MAKER1).exists());
    assert!(wallet_path.join(MAKER2).exists());

    // Create 3 taker and maker address and send 0.05 btc to each
    for _ in 0..3 {
        let taker_address = taker_wallet.get_next_external_address(&rpc).unwrap();
        let maker1_address = maker1_wallet.get_next_external_address(&rpc).unwrap();
        let maker2_address = maker2_wallet.get_next_external_address(&rpc).unwrap();

        rpc.send_to_address(
            &taker_address,
            Amount::from_btc(0.05).unwrap(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        rpc.send_to_address(
            &maker1_address,
            Amount::from_btc(0.05).unwrap(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        rpc.send_to_address(
            &maker2_address,
            Amount::from_btc(0.05).unwrap(),
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
    }

    // Create a fidelity bond for each maker
    let maker1_fbond_address = maker1_wallet
        .get_timelocked_address(&YearAndMonth::new(2030, 1))
        .0;
    let maker2_fbond_address = maker2_wallet
        .get_timelocked_address(&YearAndMonth::new(2030, 1))
        .0;
    rpc.send_to_address(
        &maker1_fbond_address,
        Amount::from_btc(0.05).unwrap(),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();
    rpc.send_to_address(
        &maker2_fbond_address,
        Amount::from_btc(0.05).unwrap(),
        None,
        None,
        None,
        None,
        None,
        None,
    )
    .unwrap();

    generate_1_block(&rpc);

    // Check inital wallet assertions
    assert_eq!(taker_wallet.get_external_index(), 3);
    assert_eq!(maker1_wallet.get_external_index(), 3);
    assert_eq!(maker2_wallet.get_external_index(), 3);

    assert_eq!(
        taker_wallet
            .list_unspent_from_wallet(&rpc, false, true)
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        maker1_wallet
            .list_unspent_from_wallet(&rpc, false, true)
            .unwrap()
            .len(),
        4
    );
    assert_eq!(
        maker2_wallet
            .list_unspent_from_wallet(&rpc, false, true)
            .unwrap()
            .len(),
        4
    );

    assert!(taker_wallet.lock_all_nonwallet_unspents(&rpc).is_ok());
    assert!(maker1_wallet.lock_all_nonwallet_unspents(&rpc).is_ok());
    assert!(maker2_wallet.lock_all_nonwallet_unspents(&rpc).is_ok());

    let kill_flag = Arc::new(RwLock::new(false));

    // Start watchtower, makers and taker to execute a coinswap
    let kill_flag_watchtower = kill_flag.clone();
    let watchtower_thread = thread::spawn(|| {
        teleport::run_watchtower(&WATCHTOWER_DATA.into(), Some(kill_flag_watchtower));
    });

    let kill_flag_maker1 = kill_flag.clone();
    let maker1_thread = thread::spawn(|| {
        teleport::run_maker(
            &MAKER1.into(),
            WalletSyncAddressAmount::Testing,
            6102,
            MakerBehavior::Normal,
            Some(kill_flag_maker1),
        );
    });

    let kill_flag_maker2 = kill_flag.clone();
    let maker2_thread = thread::spawn(|| {
        teleport::run_maker(
            &MAKER2.into(),
            WalletSyncAddressAmount::Testing,
            16102,
            MakerBehavior::Normal,
            Some(kill_flag_maker2),
        );
    });

    let taker_thread = thread::spawn(|| {
        // Wait and then start the taker
        thread::sleep(time::Duration::from_secs(20));
        teleport::run_taker(
            &TAKER.into(),
            WalletSyncAddressAmount::Testing,
            1000,
            500000,
            2,
            3,
        );
    });

    let kill_flag_block_creation_thread = kill_flag.clone();
    let rpc_ptr = Arc::new(rpc);
    let block_creation_thread = thread::spawn(move || {
        while !*kill_flag_block_creation_thread.read().unwrap() {
            thread::sleep(time::Duration::from_secs(5));
            generate_1_block(&rpc_ptr);
            println!("created block");
        }
        println!("ending block creation thread");
    });

    taker_thread.join().unwrap();
    *kill_flag.write().unwrap() = true;
    maker1_thread.join().unwrap();
    maker2_thread.join().unwrap();
    watchtower_thread.join().unwrap();
    block_creation_thread.join().unwrap();

    // Recreate the wallet
    let taker_wallet =
        Wallet::load_wallet_from_file(TAKER, Network::Regtest, WalletSyncAddressAmount::Testing)
            .unwrap();
    let maker1_wallet =
        Wallet::load_wallet_from_file(MAKER1, Network::Regtest, WalletSyncAddressAmount::Testing)
            .unwrap();
    let maker2_wallet =
        Wallet::load_wallet_from_file(MAKER2, Network::Regtest, WalletSyncAddressAmount::Testing)
            .unwrap();

    // Check assertions
    assert_eq!(taker_wallet.get_swapcoins_count(), 6);
    assert_eq!(maker1_wallet.get_swapcoins_count(), 6);
    assert_eq!(maker2_wallet.get_swapcoins_count(), 6);

    let (rpc, network) = teleport::get_bitcoin_rpc().unwrap();
    assert_eq!(network, Network::Regtest);

    let utxos = taker_wallet
        .list_unspent_from_wallet(&rpc, false, false)
        .unwrap();
    let balance: Amount = utxos
        .iter()
        .fold(Amount::ZERO, |acc, (u, _)| acc + u.amount);
    assert_eq!(utxos.len(), 6);
    assert!(balance < Amount::from_btc(0.15).unwrap());

    let utxos = maker1_wallet
        .list_unspent_from_wallet(&rpc, false, false)
        .unwrap();
    let balance: Amount = utxos
        .iter()
        .fold(Amount::ZERO, |acc, (u, _)| acc + u.amount);
    assert_eq!(utxos.len(), 6);
    assert!(balance > Amount::from_btc(0.15).unwrap());

    let utxos = maker2_wallet
        .list_unspent_from_wallet(&rpc, false, false)
        .unwrap();
    let balance: Amount = utxos
        .iter()
        .fold(Amount::ZERO, |acc, (u, _)| acc + u.amount);
    assert_eq!(utxos.len(), 6);
    assert!(balance > Amount::from_btc(0.15).unwrap());
}
