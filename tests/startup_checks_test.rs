use alloy::primitives::Address;
use serial_test::serial;
use std::str::FromStr;
use vea_validator::config::ValidatorConfig;

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Arbitrum RPC unreachable or unhealthy")]
async fn test_startup_fails_with_bad_arbitrum_rpc() {
    println!("\n==============================================");
    println!("STARTUP TEST: Bad Arbitrum RPC Should Panic");
    println!("==============================================\n");

    let mut config = ValidatorConfig::from_env().expect("Failed to load config");

    // Point to a non-existent RPC endpoint
    config.arbitrum_rpc = "http://localhost:9999".to_string();

    println!("Testing with bad Arbitrum RPC: {}", config.arbitrum_rpc);

    // This should panic with "FATAL: Arbitrum RPC unreachable or unhealthy"
    vea_validator::startup::check_rpc_health(&config).await.unwrap();
}

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Ethereum RPC unreachable or unhealthy")]
async fn test_startup_fails_with_bad_ethereum_rpc() {
    println!("\n==============================================");
    println!("STARTUP TEST: Bad Ethereum RPC Should Panic");
    println!("==============================================\n");

    let mut config = ValidatorConfig::from_env().expect("Failed to load config");

    config.ethereum_rpc = "http://localhost:9998".to_string();

    println!("Testing with bad Ethereum RPC: {}", config.ethereum_rpc);

    vea_validator::startup::check_rpc_health(&config).await.unwrap();
}

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Gnosis RPC unreachable or unhealthy")]
async fn test_startup_fails_with_bad_gnosis_rpc() {
    println!("\n==============================================");
    println!("STARTUP TEST: Bad Gnosis RPC Should Panic");
    println!("==============================================\n");

    let mut config = ValidatorConfig::from_env().expect("Failed to load config");

    config.gnosis_rpc = "http://localhost:9997".to_string();

    println!("Testing with bad Gnosis RPC: {}", config.gnosis_rpc);

    vea_validator::startup::check_rpc_health(&config).await.unwrap();
}

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Insufficient ETH balance")]
async fn test_startup_fails_with_insufficient_eth_balance() {
    println!("\n==============================================");
    println!("STARTUP TEST: Insufficient ETH Balance Should Panic");
    println!("==============================================\n");

    let config = ValidatorConfig::from_env().expect("Failed to load config");

    // Use an address with zero balance (not our funded test wallet)
    let broke_address = Address::from_str("0x0000000000000000000000000000000000000001")
        .expect("Invalid address");

    println!("Testing with broke address: {}", broke_address);
    println!("This address should have insufficient ETH for deposit");

    // This should panic with "FATAL: Insufficient ETH balance"
    vea_validator::startup::check_balances(&config, broke_address).await.unwrap();
}

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Insufficient WETH balance on Gnosis")]
async fn test_startup_fails_with_insufficient_weth_balance() {
    use alloy::providers::{ProviderBuilder, Provider};
    use alloy::signers::local::PrivateKeySigner;
    use alloy::network::EthereumWallet;
    use alloy::rpc::types::TransactionRequest;
    use alloy::primitives::U256;

    println!("\n==============================================");
    println!("STARTUP TEST: Insufficient WETH Balance Should Panic");
    println!("==============================================\n");

    let config = ValidatorConfig::from_env().expect("Failed to load config");

    // Fund a fresh address with ETH but NO WETH
    let test_wallet = Address::from_str("0x0000000000000000000000000000000000000099")
        .expect("Invalid address");

    // Use funded test account from env
    let signer = PrivateKeySigner::from_str(&config.private_key).unwrap();
    let sender_wallet = EthereumWallet::from(signer);

    let eth_with_wallet = ProviderBuilder::new()
        .wallet(sender_wallet)
        .connect_http(config.ethereum_rpc.parse().unwrap());

    // Send 2 ETH (more than enough for deposit)
    let tx = TransactionRequest::default()
        .to(test_wallet)
        .value(U256::from(2_000_000_000_000_000_000u128));

    eth_with_wallet.send_transaction(tx).await.unwrap().get_receipt().await.unwrap();

    println!("✓ Sent ETH to test address");
    println!("Testing with address: {} (has ETH, no WETH)", test_wallet);

    // This should panic with "FATAL: Insufficient WETH balance on Gnosis"
    vea_validator::startup::check_balances(&config, test_wallet).await.unwrap();
}
