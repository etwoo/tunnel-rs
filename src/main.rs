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
use std::io::{self, Write};
use std::thread;
use std::time::Duration;
use tunnel::{
    Tunnel, TunnelBuilder, TunnelBuilderChoice, TunnelCellType, TunnelIndex,
};

type Idx = u16; // for interop with crossterm::terminal::size()

struct SimpleBuilder {
    rng: ThreadRng,
}

impl TunnelBuilder for SimpleBuilder {
    fn choose_player_start<T: TunnelIndex>(&mut self, max: T) -> T {
        max / 2.into()
    }
    fn choose_step(&mut self) -> TunnelBuilderChoice {
        if self.rng.random_bool(0.5) {
            TunnelBuilderChoice::MoveLeftWall
        } else {
            TunnelBuilderChoice::MoveRightWall
        }
    }
}

fn display(t: &Tunnel<Idx>, score_row: Idx, game_score: u64) -> io::Result<()> {
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

fn demo_step(t: &Tunnel<Idx>, timeout: Duration) -> PlayerInput {
    thread::sleep(timeout);

    let mut player = 0;
    let mut safe_min = Idx::MAX;
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
        PlayerInput::MoveLeft
    } else if player < safe_goal {
        PlayerInput::MoveRight
    } else {
        PlayerInput::Empty
    }
}

fn keyboard_step(timeout: Duration) -> PlayerInput {
    if let Ok(true) = event::poll(timeout)
        && let Ok(event) = event::read()
        && let Some(key) = event.as_key_press_event()
    {
        match key.code {
            KeyCode::Char('c' | 'q') => PlayerInput::Quit,
            KeyCode::Left => PlayerInput::MoveLeft,
            KeyCode::Right => PlayerInput::MoveRight,
            _ => PlayerInput::Empty,
        }
    } else {
        PlayerInput::Empty
    }
}

#[derive(PartialEq)]
enum PlayerType {
    SelfDemo,
    Keyboard,
}

enum PlayerInput {
    Empty,
    MoveLeft,
    MoveRight,
    Quit,
}

fn main() -> io::Result<()> {
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

        if player_type == PlayerType::SelfDemo && game_score == 200 {
            game_over_message = "Demo complete!";
            break;
        }

        let player_input = match player_type {
            PlayerType::SelfDemo => demo_step(&game_state, timeout),
            PlayerType::Keyboard => keyboard_step(timeout),
        };

        match player_input {
            PlayerInput::Empty => {}
            PlayerInput::MoveLeft => game_state.move_player_left(),
            PlayerInput::MoveRight => game_state.move_player_right(),
            PlayerInput::Quit => {
                game_over_message = "Quitting ...";
                break;
            }
        }

        game_state.step(&mut level_builder);
        if game_state.is_collision() {
            game_over_message = "Game over!";
            break;
        }

        game_score += 1;
    }

    crossterm::execute!(io::stdout(), LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;

    println!("{game_over_message} Final score: {game_score}");
    Ok(())
}
