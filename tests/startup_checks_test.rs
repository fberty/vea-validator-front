use alloy::primitives::Address;
use serial_test::serial;
use std::str::FromStr;
use vea_validator::config::ValidatorConfig;

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Arbitrum RPC unreachable or unhealthy")]
async fn test_startup_fails_with_bad_arbitrum_rpc() {
    let mut config = ValidatorConfig::from_env().expect("Failed to load config");
    config.arbitrum_rpc = "http://localhost:9999".to_string();
    vea_validator::startup::check_rpc_health(&config).await.unwrap();
}

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Ethereum RPC unreachable or unhealthy")]
async fn test_startup_fails_with_bad_ethereum_rpc() {
    let mut config = ValidatorConfig::from_env().expect("Failed to load config");
    config.ethereum_rpc = "http://localhost:9998".to_string();
    vea_validator::startup::check_rpc_health(&config).await.unwrap();
}

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Gnosis RPC unreachable or unhealthy")]
async fn test_startup_fails_with_bad_gnosis_rpc() {
    let mut config = ValidatorConfig::from_env().expect("Failed to load config");
    config.gnosis_rpc = "http://localhost:9997".to_string();
    vea_validator::startup::check_rpc_health(&config).await.unwrap();
}

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Insufficient ETH balance")]
async fn test_startup_fails_with_insufficient_eth_balance() {
    let config = ValidatorConfig::from_env().expect("Failed to load config");
    let broke_address = Address::from_str("0x0000000000000000000000000000000000000001")
        .expect("Invalid address");
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

    let config = ValidatorConfig::from_env().expect("Failed to load config");
    let test_wallet = Address::from_str("0x0000000000000000000000000000000000000099")
        .expect("Invalid address");

    let signer = PrivateKeySigner::from_str(&config.private_key).unwrap();
    let sender_wallet = EthereumWallet::from(signer);
    let eth_with_wallet = ProviderBuilder::new()
        .wallet(sender_wallet)
        .connect_http(config.ethereum_rpc.parse().unwrap());

    let tx = TransactionRequest::default()
        .to(test_wallet)
        .value(U256::from(2_000_000_000_000_000_000u128));

    eth_with_wallet.send_transaction(tx).await.unwrap().get_receipt().await.unwrap();
    vea_validator::startup::check_balances(&config, test_wallet).await.unwrap();
}
