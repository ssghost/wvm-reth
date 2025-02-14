//! OP-Reth `eth_` endpoint implementation.

use alloy_primitives::{Address, U64};
use reth_chainspec::ChainInfo;
use reth_errors::RethResult;
use reth_evm::ConfigureEvm;
use reth_provider::{
    BlockReaderIdExt, ChainSpecProvider, EvmEnvProvider, HeaderProvider, StateProviderFactory,
};
use reth_rpc_eth_api::{
    helpers::{
        Call, EthApiSpec, EthBlocks, EthCall, EthFees, EthSigner, EthState, EthTransactions,
        LoadBlock, LoadFee, LoadPendingBlock, LoadReceipt, LoadState, LoadTransaction,
        SpawnBlocking, Trace,
    },
    RawTransactionForwarder,
};
use reth_rpc_eth_types::{EthStateCache, PendingBlock};
use reth_rpc_types::SyncStatus;
use reth_tasks::{pool::BlockingTaskPool, TaskSpawner};
use reth_transaction_pool::TransactionPool;
use std::future::Future;
use tokio::sync::{AcquireError, Mutex, OwnedSemaphorePermit};

/// OP-Reth `Eth` API implementation.
///
/// This type provides the functionality for handling `eth_` related requests.
///
/// This wraps a default `Eth` implementation, and provides additional functionality where the
/// optimism spec deviates from the default (ethereum) spec, e.g. transaction forwarding to the
/// sequencer, receipts, additional RPC fields for transaction receipts.
///
/// This type implements the [`FullEthApi`](reth_rpc_eth_api::helpers::FullEthApi) by implemented
/// all the `Eth` helper traits and prerequisite traits.
#[derive(Debug, Clone)]
pub struct OpEthApi<Eth> {
    inner: Eth,
}

impl<Eth> OpEthApi<Eth> {
    /// Creates a new `OpEthApi` from the provided `Eth` implementation.
    pub const fn new(inner: Eth) -> Self {
        Self { inner }
    }
}

impl<Eth: EthApiSpec> EthApiSpec for OpEthApi<Eth> {
    fn protocol_version(&self) -> impl Future<Output = RethResult<U64>> + Send {
        self.inner.protocol_version()
    }

    fn chain_id(&self) -> U64 {
        self.inner.chain_id()
    }

    fn chain_info(&self) -> RethResult<ChainInfo> {
        self.inner.chain_info()
    }

    fn accounts(&self) -> Vec<Address> {
        self.inner.accounts()
    }

    fn is_syncing(&self) -> bool {
        self.inner.is_syncing()
    }

    fn sync_status(&self) -> RethResult<SyncStatus> {
        self.inner.sync_status()
    }
}

impl<Eth: LoadBlock> LoadBlock for OpEthApi<Eth> {
    fn provider(&self) -> impl BlockReaderIdExt {
        LoadBlock::provider(&self.inner)
    }

    fn cache(&self) -> &reth_rpc_eth_types::EthStateCache {
        self.inner.cache()
    }
}

impl<Eth: LoadPendingBlock> LoadPendingBlock for OpEthApi<Eth> {
    fn provider(
        &self,
    ) -> impl BlockReaderIdExt + EvmEnvProvider + ChainSpecProvider + StateProviderFactory {
        self.inner.provider()
    }

    fn pool(&self) -> impl TransactionPool {
        self.inner.pool()
    }

    fn pending_block(&self) -> &Mutex<Option<PendingBlock>> {
        self.inner.pending_block()
    }

    fn evm_config(&self) -> &impl ConfigureEvm {
        self.inner.evm_config()
    }
}

impl<Eth: SpawnBlocking> SpawnBlocking for OpEthApi<Eth> {
    fn io_task_spawner(&self) -> impl TaskSpawner {
        self.inner.io_task_spawner()
    }

    fn tracing_task_pool(&self) -> &BlockingTaskPool {
        self.inner.tracing_task_pool()
    }

    fn acquire_owned(
        &self,
    ) -> impl Future<Output = Result<OwnedSemaphorePermit, AcquireError>> + Send {
        self.inner.acquire_owned()
    }

    fn acquire_many_owned(
        &self,
        n: u32,
    ) -> impl Future<Output = Result<OwnedSemaphorePermit, AcquireError>> + Send {
        self.inner.acquire_many_owned(n)
    }
}

impl<Eth: LoadReceipt> LoadReceipt for OpEthApi<Eth> {
    fn cache(&self) -> &EthStateCache {
        self.inner.cache()
    }
}

impl<Eth: LoadFee> LoadFee for OpEthApi<Eth> {
    fn provider(&self) -> impl reth_provider::BlockIdReader + HeaderProvider + ChainSpecProvider {
        LoadFee::provider(&self.inner)
    }

    fn cache(&self) -> &EthStateCache {
        LoadFee::cache(&self.inner)
    }

    fn gas_oracle(&self) -> &reth_rpc_eth_types::GasPriceOracle<impl BlockReaderIdExt> {
        self.inner.gas_oracle()
    }

    fn fee_history_cache(&self) -> &reth_rpc_eth_types::FeeHistoryCache {
        self.inner.fee_history_cache()
    }
}

impl<Eth: Call> Call for OpEthApi<Eth> {
    fn call_gas_limit(&self) -> u64 {
        self.inner.call_gas_limit()
    }

    fn evm_config(&self) -> &impl ConfigureEvm {
        self.inner.evm_config()
    }
}

impl<Eth: LoadState> LoadState for OpEthApi<Eth> {
    fn provider(&self) -> impl StateProviderFactory {
        LoadState::provider(&self.inner)
    }

    fn cache(&self) -> &EthStateCache {
        LoadState::cache(&self.inner)
    }

    fn pool(&self) -> impl TransactionPool {
        LoadState::pool(&self.inner)
    }
}

impl<Eth: LoadTransaction> LoadTransaction for OpEthApi<Eth> {
    type Pool = Eth::Pool;

    fn provider(&self) -> impl reth_provider::TransactionsProvider {
        LoadTransaction::provider(&self.inner)
    }

    fn cache(&self) -> &EthStateCache {
        LoadTransaction::cache(&self.inner)
    }

    fn pool(&self) -> &Self::Pool {
        LoadTransaction::pool(&self.inner)
    }
}

impl<Eth: EthTransactions> EthTransactions for OpEthApi<Eth> {
    fn provider(&self) -> impl BlockReaderIdExt {
        EthTransactions::provider(&self.inner)
    }

    fn raw_tx_forwarder(&self) -> Option<std::sync::Arc<dyn RawTransactionForwarder>> {
        self.inner.raw_tx_forwarder()
    }

    fn signers(&self) -> &parking_lot::RwLock<Vec<Box<dyn EthSigner>>> {
        self.inner.signers()
    }
}

impl<Eth: EthBlocks> EthBlocks for OpEthApi<Eth> {
    fn provider(&self) -> impl HeaderProvider {
        EthBlocks::provider(&self.inner)
    }
}

impl<Eth: EthState> EthState for OpEthApi<Eth> {
    fn max_proof_window(&self) -> u64 {
        self.inner.max_proof_window()
    }
}

impl<Eth: EthCall> EthCall for OpEthApi<Eth> {}

impl<Eth: EthFees> EthFees for OpEthApi<Eth> {}

impl<Eth: Trace> Trace for OpEthApi<Eth> {
    fn evm_config(&self) -> &impl ConfigureEvm {
        self.inner.evm_config()
    }
}
