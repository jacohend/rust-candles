use std::error::Error;
use std::{process, thread, time::Duration};
use cli_candlestick_chart::{Candle, Chart};
use serde::{Serialize, Deserialize};
use tui::{widgets::Widget, style::Modifier};
use unicode_width::UnicodeWidthStr;
use ansi_parser::AnsiParser;

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

pub struct AnsiEscape<'a>(&'a str);

impl<'a> Widget for AnsiEscape<'a> {
    fn render(self, area: tui::layout::Rect, buf: &mut tui::buffer::Buffer) {
        for (h, line) in self.0.lines().enumerate() {
            let h = area.top() + h as u16;
            let mut w = area.left();
            let mut s = tui::style::Style::default();
            for block in line.ansi_parse() {
                match block {
                    ansi_parser::Output::TextBlock(text) => {
                        if w < buf.area.width {
                            buf.set_string(w, h, text, s);
                            w += text.width() as u16;
                        }
                    }
                    ansi_parser::Output::Escape(escape) => match escape {
                        ansi_parser::AnsiSequence::SetGraphicsMode(v) => {
                            fn color(v: &[u8]) -> tui::style::Color {
                                match v[1] {
                                    2 => tui::style::Color::Rgb(v[2], v[3], v[4]),
                                    5 => tui::style::Color::Indexed(v[2]),
                                    _ => panic!("unsupport color"),
                                }
                            }

                            s = match v[0] {
                                0 => tui::style::Style::default(),
                                1 => s.add_modifier(Modifier::BOLD),
                                2 => s.remove_modifier(Modifier::BOLD),
                                38 => tui::style::Style::default().fg(color(&v)),
                                48 => tui::style::Style::default().bg(color(&v)),
                                v => panic!("unsupport attribute: {v}"),
                            };
                        }
                        ansi_parser::AnsiSequence::ResetMode(_) => {
                            s = tui::style::Style::default();
                        }
                        _ => panic!("unssport escape sequence"),
                    },
                }
            }
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {

    ctrlc::set_handler(move || {
        println!("received Ctrl+C!");
        process::exit(0x0100);
    }).expect("Error setting Ctrl-C handler");

    crossterm::terminal::enable_raw_mode()?;
    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let mut terminal = tui::Terminal::new(tui::backend::CrosstermBackend::new(stdout))?;

    loop {
        let candles =
            reqwest::blocking::get("https://api.binance.us/api/v1/klines?symbol=BTCUSDT&interval=15m")?
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
        terminal.draw(|frame| {
            let area = frame.size().inner(&tui::layout::Margin {
                vertical: 2,
                horizontal: 2,
            });
            frame.render_widget(AnsiEscape(&chart.render()), area);
        })?;
        thread::sleep(Duration::from_millis(15000));
    }
    Ok(())
}