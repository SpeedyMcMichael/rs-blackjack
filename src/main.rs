use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Terminal,
};
use std::io;

#[derive(Clone, Debug, PartialEq)]
enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

impl Suit {
    fn symbol(&self) -> &str {
        match self {
            Suit::Spades => "♠",
            Suit::Hearts => "♥",
            Suit::Diamonds => "♦",
            Suit::Clubs => "♣",
        }
    }
    fn color(&self) -> Color {
        match self {
            Suit::Hearts | Suit::Diamonds => Color::Red,
            Suit::Spades | Suit::Clubs => Color::White,
        }
    }
}

#[derive(Clone, Debug)]
struct Card {
    rank: u8,
    suit: Suit,
}

impl Card {
    fn rank_str(&self) -> &str {
        match self.rank {
            1 => "A",
            11 => "J",
            12 => "Q",
            13 => "K",
            _ => match self.rank {
                2 => "2",
                3 => "3",
                4 => "4",
                5 => "5",
                6 => "6",
                7 => "7",
                8 => "8",
                9 => "9",
                10 => "10",
                _ => "?",
            },
        }
    }

    fn value(&self) -> u8 {
        match self.rank {
            1 => 11,
            11 | 12 | 13 => 10,
            n => n,
        }
    }
}

fn hand_value(hand: &[Card]) -> u8 {
    let mut total: u8 = 0;
    let mut aces = 0u8;
    for card in hand {
        let v = card.value();
        total = total.saturating_add(v);
        if card.rank == 1 {
            aces += 1;
        }
    }
    while total > 21 && aces > 0 {
        total = total.saturating_sub(10);
        aces -= 1;
    }
    total
}

fn build_deck() -> Vec<Card> {
    let suits = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];
    let mut deck = Vec::with_capacity(52);
    for suit in &suits {
        for rank in 1u8..=13 {
            deck.push(Card {
                rank,
                suit: suit.clone(),
            });
        }
    }
    deck.shuffle(&mut thread_rng());
    deck
}

#[derive(PartialEq)]
enum Phase {
    Betting,
    Verification,
    Playing,
    DealerTurn,
    Result,
}

#[derive(PartialEq, Debug)]
enum Outcome {
    Win,
    Lose,
    Push,
    Blackjack,
    Bust,
    DealerBust,
}

struct Game {
    deck: Vec<Card>,
    player: Vec<Card>,
    dealer: Vec<Card>,
    chips: i32,
    bet: i32,
    phase: Phase,
    outcome: Option<Outcome>,
    message: String,
    bet_input: String,
    verification_input: String,
    current_balance: i32,
}

impl Game {
    fn new() -> Self {
        Game {
            deck: build_deck(),
            player: vec![],
            dealer: vec![],
            chips: 500,
            bet: 0,
            phase: Phase::Betting,
            outcome: None,
            message: String::from("Place your bet to start!"),
            bet_input: String::new(),
            verification_input: String::new(),
            current_balance: thread_rng().gen_range(1_000..=1_000_000),
        }
    }

    fn roll_fake_balance(&mut self) {
        self.current_balance = thread_rng().gen_range(1_000..=1_000_000);
    }

    fn deal(&mut self) {
        if self.deck.len() < 10 {
            self.deck = build_deck();
        }
        self.player.clear();
        self.dealer.clear();
        self.player.push(self.deck.pop().unwrap());
        self.dealer.push(self.deck.pop().unwrap());
        self.player.push(self.deck.pop().unwrap());
        self.dealer.push(self.deck.pop().unwrap());

        self.phase = Phase::Playing;
        self.outcome = None;

        if hand_value(&self.player) == 21 {
            self.phase = Phase::Result;
            self.outcome = Some(Outcome::Blackjack);
            self.chips += (self.bet as f32 * 1.5) as i32;
            self.message = format!("BLACKJACK! +{} chips", (self.bet as f32 * 1.5) as i32);
        } else {
            self.message = String::from("Hit or Stand?");
        }
    }

    fn hit(&mut self) {
        if self.phase != Phase::Playing {
            return;
        }
        self.player.push(self.deck.pop().unwrap());
        let v = hand_value(&self.player);
        if v > 21 {
            self.phase = Phase::Result;
            self.outcome = Some(Outcome::Bust);
            self.chips -= self.bet;
            self.message = format!("Bust! ({}) -{}  chips", v, self.bet);
        } else if v == 21 {
            self.stand();
        }
    }

    fn stand(&mut self) {
        if self.phase != Phase::Playing {
            return;
        }
        self.phase = Phase::DealerTurn;
        while hand_value(&self.dealer) < 17 {
            self.dealer.push(self.deck.pop().unwrap());
        }
        let pv = hand_value(&self.player);
        let dv = hand_value(&self.dealer);

        if dv > 21 {
            self.outcome = Some(Outcome::DealerBust);
            self.chips += self.bet;
            self.message = format!("Dealer busts ({})! +{} chips", dv, self.bet);
        } else if pv > dv {
            self.outcome = Some(Outcome::Win);
            self.chips += self.bet;
            self.message = format!("You win! ({} vs {}) +{} chips", pv, dv, self.bet);
        } else if dv > pv {
            self.outcome = Some(Outcome::Lose);
            self.chips -= self.bet;
            self.message = format!("Dealer wins ({} vs {}) -{} chips", dv, pv, self.bet);
        } else {
            self.outcome = Some(Outcome::Push);
            self.message = format!("Push! ({}) Bet returned", pv);
        }
        self.phase = Phase::Result;
    }

    fn confirm_bet(&mut self) {
        if let Ok(b) = self.bet_input.parse::<i32>() {
            if b == self.current_balance {
                self.bet = b;
                self.bet_input.clear();
                self.verification_input.clear();
                self.phase = Phase::Verification;
                self.message = String::from("Payment verification required before dealing.");
            } else if b < self.current_balance {
                let complaints = [
                    "Why would you bet less than your life savings?",
                    "Error, not a real gambler.",
                    "Bet your entire balance like a responsible gambler.",
                ];
                self.message = complaints
                    .choose(&mut thread_rng())
                    .unwrap_or(&"Bet your entire balance like a responsible gambler.")
                    .to_string();
            } else {
                self.message = format!(
                    "Nice ambition, but you must bet exactly your full balance: {}",
                    self.current_balance
                );
            }
        }
    }

    fn confirm_verification(&mut self) {
        if self.verification_input == "I love blackjack" {
            self.verification_input.clear();
            self.deal();
        } else {
            self.message = String::from("Verification failed. Exact phrase required.");
        }
    }
}

fn render_card_widget(card: &Card, hidden: bool) -> Vec<Line<'static>> {
    if hidden {
        vec![
            Line::from(Span::styled(
                "┌─────┐",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "│░░░░░│",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "│░░░░░│",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "│░░░░░│",
                Style::default().fg(Color::DarkGray),
            )),
            Line::from(Span::styled(
                "└─────┘",
                Style::default().fg(Color::DarkGray),
            )),
        ]
    } else {
        let r = card.rank_str().to_string();
        let s = card.suit.symbol().to_string();
        let c = card.suit.color();
        let top = format!("{:<2}   ", r);
        let bot = format!("   {:>2}", r);
        vec![
            Line::from(Span::styled("┌─────┐", Style::default().fg(Color::White))),
            Line::from(vec![
                Span::styled("│", Style::default().fg(Color::White)),
                Span::styled(top, Style::default().fg(c)),
                Span::styled("│", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("│  ", Style::default().fg(Color::White)),
                Span::styled(
                    s.clone(),
                    Style::default().fg(c).add_modifier(Modifier::BOLD),
                ),
                Span::styled("  │", Style::default().fg(Color::White)),
            ]),
            Line::from(vec![
                Span::styled("│", Style::default().fg(Color::White)),
                Span::styled(bot, Style::default().fg(c)),
                Span::styled("│", Style::default().fg(Color::White)),
            ]),
            Line::from(Span::styled("└─────┘", Style::default().fg(Color::White))),
        ]
    }
}

fn draw_hand_area(
    f: &mut ratatui::Frame,
    area: Rect,
    hand: &[Card],
    hide_second: bool,
    label: &str,
    score: Option<u8>,
) {
    let score_str = match score {
        Some(s) => format!(" ({})", s),
        None => String::new(),
    };
    let title = format!("{}{}", label, score_str);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 2,
        y: area.y + 1,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2),
    };

    let card_width = 7u16;
    let gap = 1u16;
    let mut x = inner.x;

    for (i, card) in hand.iter().enumerate() {
        let hidden = hide_second && i == 1;
        let lines = render_card_widget(card, hidden);
        for (row, line) in lines.iter().enumerate() {
            let y = inner.y + row as u16;
            if y >= inner.y + inner.height {
                break;
            }
            let para = Paragraph::new(line.clone());
            f.render_widget(
                para,
                Rect {
                    x,
                    y,
                    width: card_width,
                    height: 1,
                },
            );
        }
        x += card_width + gap;
        if x + card_width > inner.x + inner.width {
            break;
        }
    }
}

fn draw_ui(f: &mut ratatui::Frame, game: &Game) {
    let size = f.area();

    let bg = Block::default().style(Style::default().bg(Color::Rgb(0, 80, 0)));
    f.render_widget(bg, size);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(9),
            Constraint::Min(9),
            Constraint::Length(3),
            Constraint::Length(3),
        ])
        .split(size);

    let title = Paragraph::new(Span::styled(
        "♠  B L A C K J A C K  ♠",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    ))
    .alignment(Alignment::Center)
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    f.render_widget(title, chunks[0]);

    let hide = game.phase == Phase::Playing;
    let dealer_score = if hide {
        None
    } else {
        Some(hand_value(&game.dealer))
    };
    draw_hand_area(f, chunks[1], &game.dealer, hide, "Dealer", dealer_score);

    let player_score = if game.player.is_empty() {
        None
    } else {
        Some(hand_value(&game.player))
    };
    draw_hand_area(f, chunks[2], &game.player, false, "You", player_score);

    let msg_style = match &game.outcome {
        Some(Outcome::Win) | Some(Outcome::Blackjack) | Some(Outcome::DealerBust) => {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        }
        Some(Outcome::Bust) | Some(Outcome::Lose) => {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        }
        Some(Outcome::Push) => Style::default().fg(Color::Yellow),
        None => Style::default().fg(Color::White),
    };

    let msg_para = Paragraph::new(Span::styled(game.message.clone(), msg_style))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(msg_para, chunks[3]);

    let status = match game.phase {
        Phase::Betting => {
            let bet_display = if game.bet_input.is_empty() {
                "_".to_string()
            } else {
                game.bet_input.clone()
            };
            format!(
                "Current Balance: ${}  │  Chips: {}  │  Bet: {}  │  [0-9] type bet  [Enter] deal  [Q] quit",
                game.current_balance, game.chips, bet_display
            )
        }
        Phase::Playing => format!(
            "Chips: {}  │  Bet: {}  │  [H] hit  [S] stand  [Q] quit",
            game.chips, game.bet
        ),
        Phase::Verification => {
            let typed = if game.verification_input.is_empty() {
                "_"
            } else {
                &game.verification_input
            };
            format!(
                "Chips: {}  │  Bet: {}  │  Verify: {}  │  [Enter] submit  [Q] quit",
                game.chips, game.bet, typed
            )
        }
        Phase::Result | Phase::DealerTurn => {
            format!("Chips: {}  │  [N] new hand  [Q] quit", game.chips)
        }
    };

    let controls = Paragraph::new(Span::styled(status, Style::default().fg(Color::Cyan)))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        );
    f.render_widget(controls, chunks[4]);

    if game.chips <= 0 {
        let popup_area = centered_rect(40, 30, size);
        f.render_widget(Clear, popup_area);
        let popup = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "BROKE",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "You ran out of chips.",
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "[Q] quit",
                Style::default().fg(Color::DarkGray),
            )),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Red))
                .title(Span::styled(" Game Over ", Style::default().fg(Color::Red))),
        );
        f.render_widget(popup, popup_area);
    }

    if game.phase == Phase::Verification {
        let popup_area = centered_rect(56, 48, size);
        f.render_widget(Clear, popup_area);
        let popup = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "Payment Verification",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Please enter your payment",
                Style::default().fg(Color::White),
            )),
            Line::from(Span::styled(
                "Credit or debit?",
                Style::default().fg(Color::White),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Type exactly: I love blackjack",
                Style::default().fg(Color::Cyan),
            )),
            Line::from(""),
            Line::from(Span::styled(
                game.verification_input.clone(),
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )),
        ])
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Magenta))
                .title(Span::styled(
                    " Totally Legit Checkout ",
                    Style::default().fg(Color::Magenta),
                )),
        );
        f.render_widget(popup, popup_area);
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn main() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut game = Game::new();

    loop {
        terminal.draw(|f| draw_ui(f, &game))?;

        if let Event::Key(key) = event::read()? {
            match game.phase {
                Phase::Betting => match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        if game.bet_input.len() < 5 {
                            game.bet_input.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        game.bet_input.pop();
                    }
                    KeyCode::Enter => game.confirm_bet(),
                    _ => {}
                },
                Phase::Playing => match key.code {
                    KeyCode::Char('h') | KeyCode::Char('H') => game.hit(),
                    KeyCode::Char('s') | KeyCode::Char('S') => game.stand(),
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    _ => {}
                },
                Phase::Verification => match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    KeyCode::Char(c)
                        if c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '?' =>
                    {
                        if game.verification_input.len() < 40 {
                            game.verification_input.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        game.verification_input.pop();
                    }
                    KeyCode::Enter => game.confirm_verification(),
                    _ => {}
                },
                Phase::Result | Phase::DealerTurn => match key.code {
                    KeyCode::Char('n') | KeyCode::Char('N') => {
                        if game.chips > 0 {
                            game.bet = 0;
                            game.phase = Phase::Betting;
                            game.player.clear();
                            game.dealer.clear();
                            game.outcome = None;
                            game.roll_fake_balance();
                            game.message = String::from("Place your bet to start!");
                        }
                    }
                    KeyCode::Char('q') | KeyCode::Char('Q') => break,
                    _ => {}
                },
            }
        }
    }

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}
