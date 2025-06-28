use crossterm::{
    QueueableCommand, cursor,
    event::{self, KeyCode},
    style::{PrintStyledContent, Stylize},
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use rand::{Rng, rngs::ThreadRng};
use std::cmp;
use std::env;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::io::{self, Stdout, Write};
use std::thread;
use std::time::Duration;
use tunnel::{
    Tunnel, TunnelBuilder, TunnelBuilderChoice, TunnelCellType,
    TunnelCellVisitor,
};

struct SimpleBuilder {
    rng: ThreadRng,
}

impl TunnelBuilder for SimpleBuilder {
    fn choose_player_start(&mut self, max: u16) -> u16 {
        max / 2
    }
    fn choose_step(&mut self) -> TunnelBuilderChoice {
        if self.rng.random_bool(0.5) {
            TunnelBuilderChoice::MoveLeftWall
        } else {
            TunnelBuilderChoice::MoveRightWall
        }
    }
}

struct SimplePrinter {
    stdout: Stdout,
}

impl TunnelCellVisitor for SimplePrinter {
    fn visit(&mut self, x: u16, y: u16, typ: TunnelCellType) -> tunnel::Result {
        self.stdout.queue(cursor::MoveTo(y, x))?;
        match typ {
            TunnelCellType::Player => {
                self.stdout.queue(PrintStyledContent("v".green()))?;
            }
            TunnelCellType::Floor => {
                self.stdout.queue(PrintStyledContent(" ".reset()))?;
            }
            TunnelCellType::Wall => {
                self.stdout.queue(PrintStyledContent("O".reset()))?;
            }
        }
        Ok(())
    }
}

struct DemoPlayer {
    player: u16,
    safe_left: u16,
    safe_right: u16,
}

impl DemoPlayer {
    fn new() -> DemoPlayer {
        DemoPlayer {
            player: 0,
            safe_left: u16::MAX,
            safe_right: 0,
        }
    }
    fn get_safe_goal(&self) -> u16 {
        self.safe_left + self.safe_right.saturating_sub(self.safe_left) / 2
    }
}

impl TunnelCellVisitor for DemoPlayer {
    fn visit(&mut self, x: u16, y: u16, typ: TunnelCellType) -> tunnel::Result {
        if typ == TunnelCellType::Player || typ == TunnelCellType::Floor {
            if typ == TunnelCellType::Player {
                self.player = y;
            }
            if x == 1 {
                self.safe_left = cmp::min(self.safe_left, y);
                self.safe_right = cmp::max(self.safe_right, y);
            }
        }
        Ok(())
    }
}

#[derive(Debug)]
struct QuitError {}

impl Display for QuitError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "player pressed quit key")
    }
}

impl Error for QuitError {}

fn read_events(t: &mut Tunnel) -> tunnel::Result {
    if event::poll(Duration::from_secs(1))? {
        if let Ok(event) = event::read() {
            if let Some(key) = event.as_key_press_event() {
                match key.code {
                    KeyCode::Char('c' | 'q') => {
                        return Err(Box::new(QuitError {}));
                    }
                    KeyCode::Left => {
                        t.move_player_left();
                    }
                    KeyCode::Right => {
                        t.move_player_right();
                    }
                    _ => {}
                }
            }
        }
    }
    Ok(())
}

enum PlayerType {
    SelfDemo,
    Keyboard,
}

fn main() -> tunnel::Result {
    let player_type = if env::args().any(|x| x == "--demo") {
        PlayerType::SelfDemo
    } else {
        PlayerType::Keyboard
    };

    let game_over_message;
    let mut game_score = 0;

    let mut builder = SimpleBuilder { rng: rand::rng() };
    let mut printer = SimplePrinter {
        stdout: io::stdout(),
    };

    let (columns, rows) = terminal::size()?;
    terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;

    let mut t = Tunnel::new(&mut builder, rows, columns);
    loop {
        printer.stdout.queue(Clear(ClearType::All))?;
        t.accept(&mut printer)?;
        printer.stdout.queue(cursor::MoveTo(0, rows - 1))?;
        printer
            .stdout
            .queue(PrintStyledContent(format!("{game_score}").green()))?;
        printer.stdout.flush()?;

        match player_type {
            PlayerType::SelfDemo => {
                thread::sleep(Duration::from_millis(100));
                if game_score == 200 {
                    game_over_message = "Demo complete!".to_string();
                    break;
                }
                let mut demo = DemoPlayer::new();
                t.accept(&mut demo)?;
                if demo.player > demo.get_safe_goal() {
                    t.move_player_left()
                } else if demo.player < demo.get_safe_goal() {
                    t.move_player_right()
                }
            }
            PlayerType::Keyboard => {
                if let Err(e) = read_events(&mut t) {
                    game_over_message = format!("Quitting because {e} ...");
                    break;
                }
            }
        }

        t.step(&mut builder);
        if t.is_collision() {
            game_over_message = "Game over!".to_string();
            break;
        }

        game_score += 1;
    }

    crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    println!("{game_over_message} Final score: {game_score}");
    Ok(())
}
