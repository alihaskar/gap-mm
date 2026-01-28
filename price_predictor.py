"""
Fast price prediction using EMA + Linear Regression.
Uses numba for high-performance computation.
"""

import numpy as np
from numba import jit
from collections import deque
from datetime import datetime
import time


@jit(nopython=True)
def update_ema(prev_ema, new_price, period, is_first):
    """
    Update EMA incrementally with new price.
    
    Args:
        prev_ema: previous EMA value
        new_price: new price to incorporate
        period: EMA period
        is_first: if True, initialize EMA with price
    
    Returns:
        new EMA value
    """
    if is_first:
        return new_price
    
    alpha = 2.0 / (period + 1.0)
    return alpha * new_price + (1 - alpha) * prev_ema


@jit(nopython=True)
def simple_linear_regression(x, y):
    """
    Simple linear regression: y = a + b*x
    Returns: (a, b, predicted_next)
    """
    n = len(x)
    if n < 2:
        return y[-1], 0.0, y[-1]
    
    x_mean = np.mean(x)
    y_mean = np.mean(y)
    
    numerator = 0.0
    denominator = 0.0
    
    for i in range(n):
        numerator += (x[i] - x_mean) * (y[i] - y_mean)
        denominator += (x[i] - x_mean) ** 2
    
    if denominator == 0:
        return y[-1], 0.0, y[-1]
    
    b = numerator / denominator
    a = y_mean - b * x_mean
    
    # Predict next value (x = n)
    predicted_next = a + b * n
    
    return a, b, predicted_next


@jit(nopython=True)
def calculate_signal(ema_array, current_close):
    """
    Calculate fair price and signal from EMA history.
    
    Args:
        ema_array: numpy array of last 11 EMA values
        current_close: current raw close price
    
    Returns:
        (fair_price, current_ema, predicted_ema, residual, signal)
        signal: 1 = BUY, -1 = SELL, 0 = HOLD
    """
    if len(ema_array) < 11:
        return 0.0, 0.0, 0.0, 0.0, 0
    
    # Use last 10 EMAs (excluding current) for regression
    x = np.arange(10, dtype=np.float64)
    y = ema_array[:10]
    
    # Predict what current EMA should be
    a, b, predicted_ema = simple_linear_regression(x, y)
    
    # Current actual EMA
    current_ema = ema_array[10]
    
    # Residual: actual EMA - predicted EMA
    residual = current_ema - predicted_ema
    
    # Fair price = raw close + residual (matches Pine Script logic)
    fair_price = current_close + residual
    
    # Generate signal: compare fair price to current close
    if fair_price > current_close:
        signal = 1  # BUY
    elif fair_price < current_close:
        signal = -1  # SELL
    else:
        signal = 0  # HOLD
    
    return fair_price, current_ema, predicted_ema, residual, signal


class PricePredictor:
    """
    Collects spot mid prices at aligned intervals and generates signals.
    """
    
    def __init__(self, sample_interval=10, ema_period=4, queue_size=11):
        """
        Args:
            sample_interval: seconds between samples (e.g., 10 for 10s bars)
            ema_period: EMA smoothing period (default 4)
            queue_size: number of samples to keep (default 11)
        """
        self.sample_interval = sample_interval
        self.ema_period = ema_period
        self.queue_size = queue_size
        
        self.price_queue = deque(maxlen=queue_size)
        self.ema_queue = deque(maxlen=queue_size)
        self.current_ema = None
        self.last_sample_time = None
        self.current_bar_prices = []
        self.last_signal_info = None  # Store last signal for continuous printing
        
    def get_next_sample_time(self, current_time):
        """Get the next aligned sample time."""
        current_seconds = current_time.second
        seconds_into_interval = current_seconds % self.sample_interval
        
        if seconds_into_interval == 0:
            return current_time
        else:
            next_aligned = current_seconds + (self.sample_interval - seconds_into_interval)
            if next_aligned >= 60:
                # Next minute
                next_time = current_time.replace(second=0, microsecond=0)
                return next_time.replace(minute=current_time.minute + 1)
            else:
                return current_time.replace(second=next_aligned, microsecond=0)
    
    def should_sample(self, current_time):
        """Check if we should take a sample now."""
        if self.last_sample_time is None:
            # First sample - check if we're at an aligned time
            if current_time.second % self.sample_interval == 0:
                return True
            return False
        
        # Check if we've crossed into a new sample period
        elapsed = (current_time - self.last_sample_time).total_seconds()
        return elapsed >= self.sample_interval
    
    def add_price(self, price, timestamp):
        """
        Add a price update.
        
        Args:
            price: current mid price
            timestamp: datetime object
        
        Returns:
            dict with signal info (updated on new sample, or last known signal)
        """
        current_time = timestamp.replace(microsecond=0)
        
        # Collect prices for current bar
        self.current_bar_prices.append(price)
        
        # Check if we should sample
        if self.should_sample(current_time):
            # Close the bar - take the last price as close
            close_price = self.current_bar_prices[-1]
            self.price_queue.append(close_price)
            
            # Update EMA incrementally (TRUE EMA with memory)
            is_first = self.current_ema is None
            self.current_ema = update_ema(
                self.current_ema if self.current_ema is not None else close_price,
                close_price,
                self.ema_period,
                is_first
            )
            self.ema_queue.append(self.current_ema)
            
            # Reset for next bar
            self.current_bar_prices = []
            self.last_sample_time = current_time
            
            # Generate signal if we have enough data
            if len(self.ema_queue) >= self.queue_size:
                ema_array = np.array(list(self.ema_queue), dtype=np.float64)
                fair, current_ema, predicted_ema, residual, signal = calculate_signal(
                    ema_array, close_price
                )
                
                signal_text = {1: "BUY", -1: "SELL", 0: "HOLD"}[signal]
                
                self.last_signal_info = {
                    'timestamp': current_time,
                    'close_price': close_price,
                    'fair_price': fair,
                    'current_ema': current_ema,
                    'predicted_ema': predicted_ema,
                    'residual': residual,
                    'signal': signal_text,
                    'recent_prices': list(self.price_queue)[-4:],
                    'recent_emas': list(self.ema_queue)[-4:],
                    'updated': True  # Flag to indicate new sample
                }
        
        # Return current signal info (either newly updated or last known)
        if self.last_signal_info is not None:
            result = self.last_signal_info.copy()
            result['updated'] = result.get('updated', False)
            # Clear the updated flag after returning once
            if 'updated' in self.last_signal_info:
                del self.last_signal_info['updated']
            return result
        
        return None
    
