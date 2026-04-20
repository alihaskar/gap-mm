mod bybit;
mod execution;
mod private_ws;

use bybit::{start_bybit_streams, process_orderbook_updates};
use execution::{ExecutionEngine, OrderAction};
use private_ws::start_private_stream;
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

    fn start_stream(
        &self,
        py: Python,
        callback: PyObject,
        symbol: Option<String>,
        tick_size: Option<f64>,
    ) -> PyResult<()> {
        let symbol = symbol.unwrap_or_else(|| "BTCUSDT".to_string());
        let tick_size = tick_size.unwrap_or(0.10);
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
                        process_orderbook_updates(rx, tick_size, move |update| {
                            Python::with_gil(|py| {
                                let dict = PyDict::new(py);
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
                                dict.set_item("timestamp", update.timestamp).ok();
                                dict.set_item("gap_prob_resistance_up", update.gap_prob_resistance_up).ok();
                                dict.set_item("gap_distance_up", update.gap_distance_up).ok();
                                dict.set_item("gap_distance_dn", update.gap_distance_dn).ok();
                                dict.set_item("liquidity_up", update.liquidity_up).ok();
                                dict.set_item("liquidity_dn", update.liquidity_dn).ok();

                                if let Err(e) = callback.call1(py, (dict,)) {
                                    // Check if it's a KeyboardInterrupt
                                    if e.is_instance_of::<pyo3::exceptions::PyKeyboardInterrupt>(py) {
                                        eprintln!("\nKeyboardInterrupt received, exiting...");
                                        std::process::exit(0);
                                    }
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

// ============================================================================
// Execution Engine Python Bindings
// ============================================================================

#[pyclass]
struct ExecutionNode {
    runtime: Arc<tokio::runtime::Runtime>,
    engine: Arc<Mutex<Option<ExecutionEngine>>>,
    api_key: String,
    api_secret: String,
    fill_callback: Arc<Mutex<Option<PyObject>>>,
}

#[pymethods]
impl ExecutionNode {
    #[new]
    fn new(
        api_key: String,
        api_secret: String,
        symbol: String,
        market_type: Option<String>,
        tick_size: Option<f64>,
        max_position: Option<f64>,
        min_order_size: Option<f64>,
        api_base_url: Option<String>,
    ) -> PyResult<Self> {
        // Install ring crypto provider for rustls
        let _ = ring::default_provider().install_default();

        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

        let market_type = market_type.unwrap_or_else(|| "spot".to_string());
        let tick_size = tick_size.unwrap_or(0.10);
        let max_position = max_position.unwrap_or(0.01);
        let min_order_size = min_order_size.unwrap_or(0.001);

        let engine = ExecutionEngine::new(
            api_key.clone(),
            api_secret.clone(),
            symbol,
            market_type,
            tick_size,
            max_position,
            min_order_size,
            api_base_url,
        );

        Ok(ExecutionNode {
            runtime: Arc::new(runtime),
            engine: Arc::new(Mutex::new(Some(engine))),
            api_key,
            api_secret,
            fill_callback: Arc::new(Mutex::new(None)),
        })
    }

    /// Start listening for fills (private WebSocket)
    fn start_fill_listener(&self, _py: Python, callback: PyObject) -> PyResult<()> {
        let runtime = self.runtime.clone();
        let engine = self.engine.clone();
        let api_key = self.api_key.clone();
        let api_secret = self.api_secret.clone();
        let fill_callback = self.fill_callback.clone();

        // Store callback
        runtime.block_on(async {
            let mut cb = fill_callback.lock().await;
            *cb = Some(callback);
        });

        // Start private stream in background
        runtime.spawn(async move {
            match start_private_stream(&api_key, &api_secret).await {
                Ok(mut rx) => {
                    eprintln!("✓ Fill listener started");
                    
                    while let Some(fill) = rx.recv().await {
                        // Update position and get the updated position
                        let position = if let Some(eng) = engine.lock().await.as_ref() {
                            eng.position_state.process_fill(&fill).await;
                            eng.position_state.get_position(&fill.symbol).await
                        } else {
                            None
                        };

                        // Call Python callback in spawn_blocking to avoid blocking the async runtime
                        let fill_callback_clone = fill_callback.clone();
                        tokio::task::spawn_blocking(move || {
                            Python::with_gil(|py| {
                                if let Some(cb) = fill_callback_clone.blocking_lock().as_ref() {
                                    let dict = PyDict::new(py);
                                    dict.set_item("symbol", &fill.symbol).ok();
                                    dict.set_item("order_id", &fill.order_id).ok();
                                    dict.set_item("side", &fill.side).ok();
                                    dict.set_item("fill_price", fill.fill_price).ok();
                                    dict.set_item("fill_qty", fill.fill_qty).ok();
                                    dict.set_item("cum_qty", fill.cum_qty).ok();
                                    dict.set_item("avg_price", fill.avg_price).ok();
                                    dict.set_item("order_status", &fill.order_status).ok();
                                    dict.set_item("timestamp", fill.timestamp).ok();

                                    // Add position data
                                    if let Some(pos) = position {
                                        let pos_dict = PyDict::new(py);
                                        pos_dict.set_item("net_qty", pos.net_qty).ok();
                                        pos_dict.set_item("avg_entry_price", pos.avg_entry_price).ok();
                                        pos_dict.set_item("realized_pnl", pos.realized_pnl).ok();
                                        dict.set_item("position", pos_dict).ok();
                                    }

                                    if let Err(e) = cb.call1(py, (dict,)) {
                                        eprintln!("Error calling fill callback: {}", e);
                                    }
                                }
                            });
                        }).await.ok();
                    }
                }
                Err(e) => {
                    eprintln!("Failed to start fill listener: {}", e);
                }
            }
        });

        Ok(())
    }

    /// Get current position
    fn get_position(&self, py: Python, symbol: Option<String>) -> PyResult<PyObject> {
        let runtime = self.runtime.clone();
        let engine = self.engine.clone();
        let symbol = symbol.unwrap_or_else(|| "BTCUSDT".to_string());

        py.allow_threads(|| {
            runtime.block_on(async {
                let engine_guard = engine.lock().await;
                let engine = engine_guard.as_ref()
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Engine not initialized"))?;

                let position = engine.position_state.get_position(&symbol).await;

                Python::with_gil(|py| {
                    if let Some(pos) = position {
                        let dict = PyDict::new(py);
                        dict.set_item("symbol", &pos.symbol).ok();
                        dict.set_item("net_qty", pos.net_qty).ok();
                        dict.set_item("avg_entry_price", pos.avg_entry_price).ok();
                        dict.set_item("total_buy_qty", pos.total_buy_qty).ok();
                        dict.set_item("total_sell_qty", pos.total_sell_qty).ok();
                        dict.set_item("realized_pnl", pos.realized_pnl).ok();
                        Ok(dict.into())
                    } else {
                        Ok(py.None())
                    }
                })
            })
        })
    }

    /// Cancel an order by order ID
    fn cancel_order(&self, py: Python, symbol: String, order_id: String) -> PyResult<()> {
        let runtime = self.runtime.clone();
        let engine = self.engine.clone();

        py.allow_threads(|| {
            runtime.block_on(async {
                let engine_guard = engine.lock().await;
                let engine = engine_guard.as_ref()
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Engine not initialized"))?;

                engine.auth.cancel_order(&symbol, &order_id)
                    .await
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

                Ok(())
            })
        })
    }

    /// Reconcile orders based on target prices
    fn reconcile(&self, py: Python, target_bid: Option<f64>, target_ask: Option<f64>) -> PyResult<PyObject> {
        let runtime = self.runtime.clone();
        let engine = self.engine.clone();

        py.allow_threads(|| {
            runtime.block_on(async {
                let engine_guard = engine.lock().await;
                let engine = engine_guard.as_ref()
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Engine not initialized"))?;

                let result = engine.reconcile_orders(target_bid, target_ask)
                    .await
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

                Python::with_gil(|py| {
                    let dict = PyDict::new(py);

                    if let Some(bid_action) = result.bid_action {
                        dict.set_item("bid", action_to_dict(py, bid_action)).ok();
                    }

                    if let Some(ask_action) = result.ask_action {
                        dict.set_item("ask", action_to_dict(py, ask_action)).ok();
                    }

                    Ok(dict.into())
                })
            })
        })
    }

    /// Sync orders from exchange (for recovery/startup)
    fn sync_orders(&self, py: Python) -> PyResult<()> {
        let runtime = self.runtime.clone();
        let engine = self.engine.clone();

        py.allow_threads(|| {
            // This is called before the main runtime starts, so use our runtime
            runtime.block_on(async {
                let engine_guard = engine.lock().await;
                let engine = engine_guard.as_ref()
                    .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>("Engine not initialized"))?;

                engine.sync_orders()
                    .await
                    .map_err(|e| PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(e.to_string()))?;

                Ok(())
            })
        })
    }

    fn __repr__(&self) -> String {
        format!("ExecutionNode()")
    }

    fn __str__(&self) -> String {
        format!("ExecutionNode for Bybit order execution")
    }
}

fn action_to_dict(py: Python, action: OrderAction) -> PyObject {
    let dict = PyDict::new(py);
    
    match action {
        OrderAction::Submitted { order_id, price, latency_ms } => {
            dict.set_item("type", "submitted").ok();
            dict.set_item("order_id", order_id).ok();
            dict.set_item("price", price).ok();
            dict.set_item("latency_ms", latency_ms).ok();
        }
        OrderAction::Amended { order_id, old_price, new_price, latency_ms } => {
            dict.set_item("type", "amended").ok();
            dict.set_item("order_id", order_id).ok();
            dict.set_item("old_price", old_price).ok();
            dict.set_item("new_price", new_price).ok();
            dict.set_item("latency_ms", latency_ms).ok();
        }
        OrderAction::NoChange { order_id, price } => {
            dict.set_item("type", "no_change").ok();
            dict.set_item("order_id", order_id).ok();
            dict.set_item("price", price).ok();
        }
        OrderAction::Skipped { reason } => {
            dict.set_item("type", "skipped").ok();
            dict.set_item("reason", reason).ok();
        }
    }
    
    dict.into()
}

#[pymodule]
fn rust_engine(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<TradingNode>()?;
    m.add_class::<ExecutionNode>()?;
    Ok(())
}
