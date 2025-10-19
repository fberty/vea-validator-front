use alloy::primitives::Address;
use alloy::providers::{ProviderBuilder, Provider};
use alloy::signers::local::PrivateKeySigner;
use alloy::network::EthereumWallet;
use std::str::FromStr;
use crate::contracts::{IVeaOutboxArbToEth, IVeaOutboxArbToGnosis, IWETH};
use crate::config::ValidatorConfig;

pub async fn check_rpc_health(c: &ValidatorConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    println!("Checking RPC endpoint health...");
    let arb_provider = ProviderBuilder::new().connect_http(c.arbitrum_rpc.parse()?);
    let eth_provider = ProviderBuilder::new().connect_http(c.ethereum_rpc.parse()?);
    let gnosis_provider = ProviderBuilder::new().connect_http(c.gnosis_rpc.parse()?);
    let arb_block = arb_provider.get_block_number().await
        .map_err(|e| panic!("FATAL: Arbitrum RPC unreachable or unhealthy: {}", e))?;
    println!("✓ Arbitrum RPC healthy (block: {})", arb_block);
    let eth_block = eth_provider.get_block_number().await
        .map_err(|e| panic!("FATAL: Ethereum RPC unreachable or unhealthy: {}", e))?;
    println!("✓ Ethereum RPC healthy (block: {})", eth_block);
    let gnosis_block = gnosis_provider.get_block_number().await
        .map_err(|e| panic!("FATAL: Gnosis RPC unreachable or unhealthy: {}", e))?;
    println!("✓ Gnosis RPC healthy (block: {})", gnosis_block);
    Ok(())
}

pub async fn check_balances(c: &ValidatorConfig, wallet: Address) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let signer = PrivateKeySigner::from_str(&c.private_key)?;
    let eth_providers = crate::config::setup_providers(c.ethereum_rpc.clone(), c.arbitrum_rpc.clone(), EthereumWallet::from(signer.clone()))?;
    let gnosis_providers = crate::config::setup_providers(c.gnosis_rpc.clone(), c.arbitrum_rpc.clone(), EthereumWallet::from(signer))?;
    let eth_outbox = IVeaOutboxArbToEth::new(c.outbox_arb_to_eth, eth_providers.destination_provider.clone());
    let gnosis_outbox = IVeaOutboxArbToGnosis::new(c.outbox_arb_to_gnosis, gnosis_providers.destination_provider.clone());
    let eth_deposit = eth_outbox.deposit().call().await?;
    let eth_balance = eth_providers.destination_provider.get_balance(wallet).await?;
    if eth_balance < eth_deposit {
        panic!("FATAL: Insufficient ETH balance. Need {} wei for deposit, have {} wei", eth_deposit, eth_balance);
    }
    let gnosis_deposit = gnosis_outbox.deposit().call().await?;
    let weth = IWETH::new(c.weth_gnosis, gnosis_providers.destination_provider.clone());
    let weth_balance = weth.balanceOf(wallet).call().await?;
    if weth_balance < gnosis_deposit {
        panic!("FATAL: Insufficient WETH balance on Gnosis. Need {} wei for deposit, have {} wei", gnosis_deposit, weth_balance);
    }
    println!("✓ Balance check passed: ETH={} wei, WETH={} wei", eth_balance, weth_balance);
    Ok(())
}
