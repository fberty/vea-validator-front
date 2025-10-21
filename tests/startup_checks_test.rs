use serial_test::serial;
use vea_validator::config::ValidatorConfig;

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Arbitrum RPC unreachable or unhealthy")]
async fn test_startup_fails_with_bad_arbitrum_rpc() {
    let mut c = ValidatorConfig::from_env().unwrap();
    c.chains.get_mut(&42161).unwrap().rpc_url = "http://localhost:9999".into();
    vea_validator::startup::check_rpc_health(&c).await.unwrap();
}

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Ethereum RPC unreachable or unhealthy")]
async fn test_startup_fails_with_bad_ethereum_rpc() {
    let mut c = ValidatorConfig::from_env().unwrap();
    c.chains.get_mut(&1).unwrap().rpc_url = "http://localhost:9998".into();
    vea_validator::startup::check_rpc_health(&c).await.unwrap();
}

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Gnosis RPC unreachable or unhealthy")]
async fn test_startup_fails_with_bad_gnosis_rpc() {
    let mut c = ValidatorConfig::from_env().unwrap();
    c.chains.get_mut(&100).unwrap().rpc_url = "http://localhost:9997".into();
    vea_validator::startup::check_rpc_health(&c).await.unwrap();
}

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Insufficient ETH balance")]
async fn test_startup_fails_with_insufficient_eth_balance() {
    use alloy::signers::local::PrivateKeySigner;
    let mut c = ValidatorConfig::from_env().unwrap();
    let broke_signer = PrivateKeySigner::from_slice(&[1u8; 32]).unwrap();
    c.wallet = alloy::network::EthereumWallet::from(broke_signer);
    vea_validator::startup::check_balances(&c).await.unwrap();
}

#[tokio::test]
#[serial]
#[should_panic(expected = "FATAL: Insufficient WETH balance on Gnosis")]
async fn test_startup_fails_with_insufficient_weth_balance() {
    use alloy::signers::local::PrivateKeySigner;
    let mut c = ValidatorConfig::from_env().unwrap();
    let broke_signer = PrivateKeySigner::from_slice(&[2u8; 32]).unwrap();
    c.wallet = alloy::network::EthereumWallet::from(broke_signer);
    vea_validator::startup::check_balances(&c).await.unwrap();
}
