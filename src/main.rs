// ANCHOR: all
mod errors;
mod tui;

use binance::api::*;
use binance::market::*;
use binance::rest_model::SymbolPrice;
use color_eyre::eyre::Result;
use crossterm::event::KeyCode::*;
use env_logger::Builder;
use log::LevelFilter;
use ratatui::{prelude::*, style::palette::tailwind, style::Modifier, widgets::*};
use tokio::sync::mpsc::{self};
use tui::Event;
use tui_textarea::{Input, Key, TextArea};
use unicode_width::UnicodeWidthStr;

const ITEM_HEIGHT: usize = 4;
const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];
const INFO_TEXT: &str =
    "(Esc) quit | (↑) move up | (↓) move down | (→) next color | (←) previous color | (s) search coin | (r) refresh";

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_style_fg: color.c400,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

// App state
struct App<'a> {
    should_quit: bool,
    longest_item_lens: (u16, u16),
    market_data: Vec<SymbolPrice>,
    state: TableState,
    scroll_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
    textarea: TextArea<'a>,
    focus_textarea: bool,
    search_coin: Option<String>,
}

impl<'a> App<'a> {
    async fn new() -> Result<Self> {
        let market_data = market_data(None).await.unwrap();
        let mut textarea = TextArea::default();
        textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::LightBlue))
                .title("Coin Search - Enter to search"),
        );
        textarea.set_style(Style::default().fg(Color::Yellow));
        textarea.set_placeholder_style(Style::default());
        textarea.set_placeholder_text("BTC/ETH/AKT \n(only 1 coin at a time without punctuation)");
        Ok(Self {
            state: TableState::default().with_selected(0),
            longest_item_lens: constraint_len_calculator(&market_data),
            scroll_state: ScrollbarState::new((market_data.len() - 1) * ITEM_HEIGHT),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            market_data,
            should_quit: false,
            textarea,
            focus_textarea: false,
            search_coin: None,
        })
    }
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.market_data.len().saturating_sub(1) {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.market_data.len().saturating_sub(1)
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn next_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTES.len();
    }

    pub fn previous_color(&mut self) {
        let count = PALETTES.len();
        self.color_index = (self.color_index + count - 1) % count;
    }

    pub fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }
}

// App actions
// ANCHOR: action_enum
#[derive(Clone)]
pub enum Action {
    NEXT,
    PREVIOUS,
    NextColor,
    PreviousColor,
    SearchFocus,
    SearchCoin(String),
    Refresh,
    Tick,
    Increment,
    Decrement,
    Quit,
    Render,
    None,
}
// ANCHOR_END: action_enum

// App ui render function
fn ui(f: &mut Frame, app: &mut App) {
    let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(f.size());

    app.set_colors();

    if app.focus_textarea {
        render_textarea(f, app);
    } else {
        render_table(f, app, rects[0]);

        render_scrollbar(f, app, rects[0]);

        render_footer(f, app, rects[1]);
    }
}

fn centered_rect(r: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Min(4),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Min(45),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_textarea(f: &mut Frame, app: &mut App) {
    let area = centered_rect(f.size(), 20, 20);
    f.render_widget(app.textarea.widget(), area);
}

fn render_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_style = Style::default()
        .fg(app.colors.header_fg)
        .bg(app.colors.header_bg);
    let selected_style = Style::default()
        .add_modifier(Modifier::REVERSED)
        .fg(app.colors.selected_style_fg);

    let header = ["Symbol", "Price"]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(1);
    let rows = app.market_data.iter().enumerate().map(|(i, data)| {
        let color = match i % 2 {
            0 => app.colors.normal_row_color,
            _ => app.colors.alt_row_color,
        };
        let item = [data.symbol.as_str(), &data.price.to_string()];
        item.into_iter()
            .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
            .collect::<Row>()
            .style(Style::new().fg(app.colors.row_fg).bg(color))
            .height(3)
    });
    let bar = " █ ";
    let t = Table::new(
        rows,
        [
            // + 1 is for padding.
            Constraint::Min(app.longest_item_lens.0 + 1),
            Constraint::Min(app.longest_item_lens.1 + 1),
        ],
    )
    .header(header)
    .highlight_style(selected_style)
    .highlight_symbol(Text::from(vec![
        "".into(),
        bar.into(),
        bar.into(),
        "".into(),
    ]))
    .bg(app.colors.buffer_bg)
    .highlight_spacing(HighlightSpacing::Always);
    f.render_stateful_widget(t, area, &mut app.state);
}

fn render_scrollbar(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None),
        area.inner(&Margin {
            vertical: 1,
            horizontal: 1,
        }),
        &mut app.scroll_state,
    );
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let info_footer = Paragraph::new(Line::from(INFO_TEXT))
        .style(Style::new().fg(app.colors.row_fg).bg(app.colors.buffer_bg))
        .centered()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::new().fg(app.colors.footer_border_color))
                .border_type(BorderType::Double),
        );
    f.render_widget(info_footer, area);
}

fn constraint_len_calculator(items: &[SymbolPrice]) -> (u16, u16) {
    let symbols = items
        .iter()
        .map(|x| x.symbol.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let string_price = items
        .iter()
        .map(|x| x.price.to_string())
        .collect::<Vec<String>>();

    let price = string_price
        .iter()
        .map(|x| x.as_str())
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (symbols as u16, price as u16)
}

// ANCHOR: get_action
fn get_action(_app: &App, event: Event) -> Action {
    match event {
        Event::Error => Action::None,
        Event::Tick => Action::Tick,
        Event::Render => Action::Render,
        Event::Key(key) => {
            match key.code {
                Char('q') | Esc => Action::Quit,
                Char('j') | Down => Action::NEXT,
                Char('k') | Up => Action::PREVIOUS,
                Char('l') | Right => Action::NextColor,
                Char('h') | Left => Action::PreviousColor,
                Char('s') => Action::SearchFocus,
                Char('r') => Action::Refresh,
                _ => Action::None,
            }
        }
        _ => Action::None,
    }
}
// ANCHOR_END: get_action

// ANCHOR: update
async fn update(app: &mut App<'_>, action: Action) {
    match action {
        Action::NEXT => {
            app.next();
        }
        Action::PREVIOUS => {
            app.previous();
        }
        Action::NextColor => {
            app.next_color();
        }
        Action::PreviousColor => {
            app.previous_color();
        }
        Action::SearchFocus => {
            app.focus_textarea = true;
        }
        Action::Refresh => {
            app.market_data = market_data(app.search_coin.as_ref()).await.unwrap();
            app.scroll_state = app
                .scroll_state
                .content_length((app.market_data.len().saturating_sub(1)) * ITEM_HEIGHT);
            app.longest_item_lens = constraint_len_calculator(&app.market_data);
            app.state = app.state.clone().with_selected(0);
        }
        Action::SearchCoin(coin) => {
            app.search_coin = if coin.is_empty() { None } else { Some(coin) };
            app.market_data = market_data(app.search_coin.as_ref()).await.unwrap();
            app.scroll_state = app
                .scroll_state
                .content_length((app.market_data.len().saturating_sub(1)) * ITEM_HEIGHT);
            app.longest_item_lens = constraint_len_calculator(&app.market_data);
            app.state = app.state.clone().with_selected(0);
        }
        Action::Quit => app.should_quit = true,
        _ => {}
    };
}
// ANCHOR_END: update

// ANCHOR: run
async fn run() -> Result<()> {
    let (action_tx, mut action_rx) = mpsc::unbounded_channel(); // new

    // ratatui terminal
    let mut tui = tui::Tui::new()?.tick_rate(1.0).frame_rate(30.0);
    tui.enter()?;
    // application state
    let mut app = App::new().await?;
    loop {
        let e = tui.next().await?;
        match e {
            tui::Event::Quit => action_tx.send(Action::Quit)?,
            tui::Event::Tick => action_tx.send(Action::Tick)?,
            tui::Event::Render => action_tx.send(Action::Render)?,
            tui::Event::Key(_) => {
                if app.focus_textarea {
                    match e.into() {
                        Input {
                            key: Key::Esc | Key::Enter,
                            ..
                        } => {
                            app.focus_textarea = false;
                            action_tx.send(Action::SearchCoin(
                                app.textarea.lines()[0].trim().to_uppercase().to_owned(),
                            ))?;
                        }
                        input => {
                            app.textarea.input(input);
                        }
                    }
                } else {
                    let action = get_action(&app, e);
                    action_tx.send(action.clone())?;
                }
            }
            _ => {}
        };

        while let Ok(action) = action_rx.try_recv() {
            // application update
            update(&mut app, action.clone()).await;
            // render only when we receive Action::Render
            if let Action::Render = action {
                tui.draw(|f| {
                    ui(f, &mut app);
                })?;
            }
        }

        // application exit
        if app.should_quit {
            break;
        }
    }
    tui.exit()?;

    Ok(())
}
// ANCHOR_END: run

#[tokio::main]
async fn main() -> Result<()> {
    errors::install_hooks()?;
    Builder::new()
        .filter_level(LevelFilter::Info)
        .parse_default_env()
        .init();
    let result = run().await;

    result?;

    Ok(())
}
// ANCHOR_END: all

async fn market_data(coin: Option<&String>) -> Result<Vec<SymbolPrice>> {
    let market: Market = Binance::new(None, None);
    // Latest price for ALL symbols with USDT as the quote asset
    match market.get_all_prices().await {
        Ok(answer) => {
            let binance::rest_model::Prices::AllPrices(all_symbols) = answer.clone();
            let coin_by_usdt = all_symbols
                .into_iter()
                .filter(|x| match coin {
                    Some(coin) => x.symbol.starts_with(coin) && x.symbol.ends_with("USDT"),
                    None => x.symbol.ends_with("USDT"),
                })
                .collect::<Vec<SymbolPrice>>();
            // info!("{:#?}", coin_by_usdt);
            Ok(coin_by_usdt)
        }
        Err(e) => {
            Err(color_eyre::eyre::eyre!(
                "Unable to get market data: {:#?}",
                e
            )) // Use the eyre macro to create the error
        }
    }
}
