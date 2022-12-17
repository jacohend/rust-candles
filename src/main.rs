use std::error::Error;
use std::{process, thread, time::Duration};
use cli_candlestick_chart::{Candle, Chart};
use tui::{widgets::Widget, style::Modifier};
use unicode_width::UnicodeWidthStr;
use ansi_parser::AnsiParser;
use tui::layout::{Constraint, Corner, Direction, Layout, Rect};
use tui::widgets::{Block, Borders, List, ListItem, Row, Table, Cell};
use binance::api::*;
use binance::config::Config;
use binance::market::*;
use tui::style::{Color, Style};
use tui::text::Spans;
use std::string::String;

const symbol: &str = "BTCUSDT";

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

    let mut stdout = std::io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let mut terminal = tui::Terminal::new(tui::backend::CrosstermBackend::new(stdout))?;

    ctrlc::set_handler(move || {
        let mut stdout = std::io::stdout();
        crossterm::execute!(stdout, crossterm::terminal::LeaveAlternateScreen);
        println!("received Ctrl+C!");
        process::exit(0x0100);
    }).expect("Error setting Ctrl-C handler");

    let config = Config::default().set_rest_api_endpoint("https://api.binance.us");
    let market: Market = Binance::new_with_config(None, None, &config);
    loop {
        let kline_summary = market.get_klines(symbol, "15m", 500, None, None).unwrap();
        let binance::model::KlineSummaries::AllKlineSummaries(klines) = kline_summary;
        let binance_candles = klines.iter().map(|candle| {
            Candle::new(
                candle.open.parse::<f64>().unwrap(),
                candle.high.parse::<f64>().unwrap(),
                candle.low.parse::<f64>().unwrap(),
                candle.close.parse::<f64>().unwrap(),
                Some(candle.volume.parse::<f64>().unwrap()),
                Some(candle.open_time as i64),
            )
        }).collect::<Vec<Candle>>();

        let all_prices = market.get_all_prices().unwrap();
        let binance::model::Prices::AllPrices(prices) = all_prices;
        let prices: Vec<(String, String)> = prices.iter().map(|price| {
            (price.symbol.to_string(), price.price.to_string())
        }).collect();
        let prices_list: Vec<Vec<(String,String)>> = prices.chunks(prices.len()/2).map(|s| s.into()).collect();

        let custom_depth = market.get_custom_depth(symbol, 100).unwrap();
        let asks:  Vec<(String, String)> = custom_depth.asks.iter().map(|a| {
            (a.price.to_string(), a.qty.to_string())
        }).collect();
        let bids:  Vec<(String, String)> = custom_depth.bids.iter().map(|a| {
            (a.price.to_string(), a.qty.to_string())
        }).collect();

        terminal.draw(|frame| {
            let area = frame.size().inner(&tui::layout::Margin {
                vertical: 0,
                horizontal: 0,
            });
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(0)
                .constraints(
                    [
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ]
                        .as_ref(),
                )
                .split(area);
            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(0)
                .constraints(
                    [
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ]
                        .as_ref(),
                )
                .split(chunks[1]);
            let bottom_left_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ]
                        .as_ref(),
                )
                .split(bottom_chunks[0]);
            let bottom_right_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints(
                    [
                        Constraint::Percentage(50),
                        Constraint::Percentage(50),
                    ]
                        .as_ref(),
                )
                .split(bottom_chunks[1]);
            let mut binance_chart = make_chart(binance_candles, chunks[0]);
            let render = binance_chart.render();
            let top_pane = AnsiEscape(&render);
            let bottom_left_pane =
                Block::default().title("Orderbook").borders(Borders::ALL);
            let bottom_right_pane =
                Block::default().title("Prices").borders(Borders::ALL);
            let asks_list = make_table("Asks".to_string(), &["Price", "Quantity"], asks, Color::Rgb(255, 0, 0));
            let bids_list = make_table("Bids".to_string(), &["Price", "Quantity"], bids, Color::Rgb(0, 255, 0));

            let prices_1_list = make_table("".to_string(), &["Symbol", "Last Price"], prices_list[0].clone(), Color::Blue);
            let prices_2_list = make_table("".to_string(), &["Symbol", "Last Price"], prices_list[1].clone(), Color::Blue);
            frame.render_widget(top_pane, area);
            frame.render_widget(bottom_left_pane, bottom_chunks[0]);
            frame.render_widget(bottom_right_pane, bottom_chunks[1]);
            frame.render_widget(bids_list, bottom_left_chunks[0]);
            frame.render_widget(asks_list, bottom_left_chunks[1]);
            frame.render_widget(prices_1_list, bottom_right_chunks[0]);
            frame.render_widget(prices_2_list, bottom_right_chunks[1]);

        })?;

        thread::sleep(Duration::from_millis(5000));
    }

    Ok(())
}

fn make_chart(candles : Vec<Candle>, area: Rect) -> Chart {
    let mut chart = Chart::new_with_size(candles, (area.width, area.height));
    chart.set_name(String::from("BTC/USD"));
    chart.set_bull_color(0, 255, 0);
    chart.set_bear_color(255, 0, 0);
    chart.set_vol_bull_color(0, 255, 0);
    chart.set_vol_bear_color(255, 0, 0);
    chart.set_volume_pane_height(4);
    chart.set_volume_pane_enabled(true);
    chart
}

fn make_table<'a>(title: String, headers: &'a [&'a str], rows : Vec<(String, String)>, c : Color) -> Table<'a> {
    let selected_style = Style::default().add_modifier(Modifier::REVERSED);
    let normal_style = Style::default().bg(Color::Black);
    let header_cells = headers
        .iter()
        .map(|h| Cell::from(*h).style(Style::default().fg(c)));
    let header = Row::new(header_cells)
        .style(normal_style)
        .height(1);
    let rows = rows.iter().map(|item| {

        let cells = vec![Cell::from(item.0.clone()), Cell::from(item.1.clone())];
        Row::new(cells)
    });
    let t = Table::new(rows)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(Spans::from(title)))
        .style(Style::default().fg(c))
        .highlight_style(selected_style)
        .highlight_symbol(">> ")
        .widths(&[
            Constraint::Percentage(50),
            Constraint::Length(30),
            Constraint::Min(10),
        ]);
    t
}
