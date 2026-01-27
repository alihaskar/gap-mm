mod bybit;

use bybit::{start_bybit_streams, process_orderbook_updates};
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rustls::crypto::ring;
use std::sync::Arc;
use tokio::sync::Mutex;

#[pyclass]
struct TradingNode {
    runtime: Arc<tokio::runtime::Runtime>,
    is_running: Arc<Mutex<bool>>,
}

#[pymethods]
impl TradingNode {
    #[new]
    fn new() -> PyResult<Self> {
        // Install ring crypto provider for rustls
        let _ = ring::default_provider().install_default();

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        Ok(TradingNode {
            runtime: Arc::new(runtime),
            is_running: Arc::new(Mutex::new(false)),
        })
    }

    fn start_stream(&self, py: Python, callback: PyObject, symbol: Option<String>) -> PyResult<()> {
        let symbol = symbol.unwrap_or_else(|| "BTCUSDT".to_string());
        let runtime = self.runtime.clone();
        let is_running = self.is_running.clone();

        // Check if already running
        let running = runtime.block_on(async {
            let running = is_running.lock().await;
            *running
        });

        if running {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Stream is already running",
            ));
        }

        // Mark as running
        runtime.block_on(async {
            let mut running = is_running.lock().await;
            *running = true;
        });

        // Release GIL and start async task
        py.allow_threads(|| {
            runtime.block_on(async {
                match start_bybit_streams(&symbol).await {
                    Ok(rx) => {
                        // Process updates and call Python callback with enriched data
                        process_orderbook_updates(rx, move |update| {
                            Python::with_gil(|py| {
                                let dict = PyDict::new_bound(py);
                                dict.set_item("symbol", &update.symbol).ok();
                                dict.set_item("source", &update.source).ok();
                                dict.set_item("bid", update.bid).ok();
                                dict.set_item("ask", update.ask).ok();
                                dict.set_item("spread", update.spread).ok();
                                dict.set_item("mid_price", update.mid_price).ok();
                                dict.set_item("spread_bps", update.spread_bps).ok();
                                dict.set_item("imbalance", update.imbalance).ok();
                                dict.set_item("bid_depth_5", update.bid_depth_5).ok();
                                dict.set_item("ask_depth_5", update.ask_depth_5).ok();

                                if let Err(e) = callback.call1(py, (dict,)) {
                                    eprintln!("Error calling Python callback: {}", e);
                                }
                            });
                        })
                        .await;
                    }
                    Err(e) => {
                        eprintln!("Error starting streams: {}", e);
                    }
                }
            });
        });

        Ok(())
    }

    fn stop(&self) -> PyResult<()> {
        let runtime = self.runtime.clone();
        let is_running = self.is_running.clone();

        runtime.block_on(async {
            let mut running = is_running.lock().await;
            *running = false;
        });

        Ok(())
    }

    fn is_running(&self) -> PyResult<bool> {
        let runtime = self.runtime.clone();
        let is_running = self.is_running.clone();

        let running = runtime.block_on(async {
            let running = is_running.lock().await;
            *running
        });

        Ok(running)
    }

    fn __repr__(&self) -> String {
        format!("TradingNode()")
    }

    fn __str__(&self) -> String {
        format!("TradingNode for Bybit orderbook streaming")
    }
}

#[pymodule]
fn rust_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TradingNode>()?;
    Ok(())
}
