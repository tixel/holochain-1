use crate::holochain_types::dna::wasm::DnaWasm;
use crate::holochain_types::dna::wasm::DnaWasmHashed;
use holo_hash::WasmHash;
use holochain_lmdb::buffer::CasBufFreshAsync;
use holochain_lmdb::error::DatabaseError;
use holochain_lmdb::error::DatabaseResult;
use holochain_lmdb::exports::SingleStore;
use holochain_lmdb::prelude::BufferedStore;
use holochain_lmdb::prelude::EnvironmentRead;
use holochain_lmdb::transaction::Writer;

/// This is where wasm lives
pub struct WasmBuf(CasBufFreshAsync<DnaWasm>);

impl WasmBuf {
    pub fn new(env: EnvironmentRead, wasm_store: SingleStore) -> DatabaseResult<Self> {
        Ok(Self(CasBufFreshAsync::new(env, wasm_store)))
    }

    pub async fn get(&self, wasm_hash: &WasmHash) -> DatabaseResult<Option<DnaWasmHashed>> {
        self.0.get(&wasm_hash).await
    }

    pub fn put(&mut self, v: DnaWasmHashed) {
        self.0.put(v);
    }
}

impl BufferedStore for WasmBuf {
    type Error = DatabaseError;

    fn flush_to_txn_ref(&mut self, writer: &mut Writer) -> DatabaseResult<()> {
        self.0.flush_to_txn_ref(writer)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::holochain_types::dna::wasm::DnaWasm;
    use holo_hash::HasHash;

    #[tokio::test(threaded_scheduler)]
    async fn wasm_store_round_trip() -> DatabaseResult<()> {
        use holochain_lmdb::prelude::*;
        observability::test_run().ok();

        // all the stuff needed to have a WasmBuf
        let env = holochain_lmdb::test_utils::test_wasm_env();
        let mut wasm_buf = WasmBuf::new(
            env.env().into(),
            env.get_db(&*holochain_lmdb::db::WASM).unwrap(),
        )
        .unwrap();

        // a wasm
        let wasm = DnaWasmHashed::from_content(DnaWasm::from(
            crate::holochain_wasm_test_utils::TestWasm::Foo,
        ))
        .await;

        // a wasm in the WasmBuf
        wasm_buf.put(wasm.clone());
        // a wasm from the WasmBuf
        let ret = wasm_buf.get(&wasm.as_hash()).await.unwrap().unwrap();

        // assert the round trip
        assert_eq!(ret, wasm);

        Ok(())
    }
}