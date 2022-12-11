use std::error::Error;
use std::{thread, time::Duration};
use cli_candlestick_chart::{Candle, Chart};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BinanceKlinesItem {
    open_time: u64,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
    close_time: u64,
    quote_asset_volume: String,
    number_of_trades: u64,
    taker_buy_base_asset_volume: String,
    taker_buy_quote_asset_volume: String,
    ignore: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    loop {
        let candles =
            reqwest::blocking::get("https://api.binance.us/api/v1/klines?symbol=BTCUSDT&interval=1m")?
                .json::<Vec<BinanceKlinesItem>>()?
                .iter()
                .map(|candle| {
                    Candle::new(
                        candle.open.parse::<f64>().unwrap(),
                        candle.high.parse::<f64>().unwrap(),
                        candle.low.parse::<f64>().unwrap(),
                        candle.close.parse::<f64>().unwrap(),
                        Some(candle.volume.parse::<f64>().unwrap()),
                        Some(candle.open_time as i64),
                    )
                })
                .collect::<Vec<Candle>>();

        let mut chart = Chart::new(&candles);

        chart.set_name(String::from("BTC/USDT"));
        chart.set_bull_color(1, 205, 254);
        chart.set_bear_color(255, 107, 153);
        chart.set_vol_bull_color(1, 205, 254);
        chart.set_vol_bear_color(255, 107, 153);
        chart.set_volume_pane_height(4);
        chart.set_volume_pane_enabled(true);
        // chart.set_volume_pane_unicode_fill(true);
        print!("{}[2J", 27 as char); //clear terminal for new draw
        chart.draw();
        thread::sleep(Duration::from_millis(15000));
    }
    Ok(())
}