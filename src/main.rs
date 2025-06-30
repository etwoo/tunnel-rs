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
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use tunnel::{Tunnel, TunnelBuilder, TunnelBuilderChoice, TunnelCellType};

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

fn display(t: &Tunnel, score_row: u16, game_score: u64) -> tunnel::Result {
    let mut stdout = io::stdout();
    stdout.queue(Clear(ClearType::All))?;
    for (row, col, cell_type) in t.iter() {
        stdout.queue(cursor::MoveTo(col, row))?;
        match cell_type {
            TunnelCellType::Player => {
                stdout.queue(PrintStyledContent("v".green()))?;
            }
            TunnelCellType::Floor => {
                stdout.queue(PrintStyledContent(" ".reset()))?;
            }
            TunnelCellType::Wall => {
                stdout.queue(PrintStyledContent("O".reset()))?;
            }
        }
    }
    stdout.queue(cursor::MoveTo(0, score_row))?;
    stdout.queue(PrintStyledContent(format!("{game_score}").green()))?;
    stdout.flush()?;
    Ok(())
}

fn demo_step(t: &mut Tunnel, timeout: Duration) {
    thread::sleep(timeout);

    let mut player = 0;
    let mut safe_min = u16::MAX;
    let mut safe_max = 0;

    for (row, col, cell_type) in t.iter() {
        if cell_type == TunnelCellType::Player {
            player = col;
        }
        if row == 1
            && (cell_type == TunnelCellType::Player
                || cell_type == TunnelCellType::Floor)
        {
            safe_min = cmp::min(safe_min, col);
            safe_max = cmp::max(safe_max, col);
        }
    }

    let safe_goal = safe_min + safe_max.saturating_sub(safe_min) / 2;

    if player > safe_goal {
        t.move_player_left()
    } else if player < safe_goal {
        t.move_player_right()
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

fn keyboard_step(t: &mut Tunnel, timeout: Duration) -> tunnel::Result {
    if event::poll(timeout)?
        && let Ok(event) = event::read()
        && let Some(key) = event.as_key_press_event()
    {
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
    Ok(())
}

enum PlayerType {
    SelfDemo,
    Keyboard,
}

fn main() -> tunnel::Result {
    let (player_type, timeout) = if env::args().any(|x| x == "--demo") {
        (PlayerType::SelfDemo, Duration::from_millis(100))
    } else {
        (PlayerType::Keyboard, Duration::from_secs(1))
    };

    let game_over_message;
    let mut game_score = 0;

    let mut level_builder = SimpleBuilder { rng: rand::rng() };

    let (columns, rows) = terminal::size()?;
    terminal::enable_raw_mode()?;
    crossterm::execute!(io::stdout(), EnterAlternateScreen)?;

    let mut game_state = Tunnel::new(&mut level_builder, rows, columns);
    loop {
        display(&game_state, rows - 1, game_score)?;

        match player_type {
            PlayerType::SelfDemo => {
                if game_score == 200 {
                    game_over_message = "Demo complete!".to_string();
                    break;
                }
                demo_step(&mut game_state, timeout);
            }
            PlayerType::Keyboard => {
                if let Err(e) = keyboard_step(&mut game_state, timeout) {
                    game_over_message = format!("Quitting because {e} ...");
                    break;
                }
            }
        }

        game_state.step(&mut level_builder);
        if game_state.is_collision() {
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
