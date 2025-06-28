// https://github.com/taiki-e/cargo-llvm-cov#exclude-code-from-coverage
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use std::collections::VecDeque;

pub type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn rows_to_loop_iterations(rows: u16) -> u16 {
    rows.saturating_sub(3)
}

#[derive(Clone)]
struct Walls {
    left_wall: u16,
    gap_to_right_wall: u16,
}

impl Walls {
    fn in_wall(&self, column: u16) -> bool {
        column <= self.left_wall
            || column > self.left_wall + self.gap_to_right_wall
    }
}

pub struct Tunnel {
    player: u16,
    screen_width: u16,
    walls: VecDeque<Walls>,
}

impl Tunnel {
    pub fn new(b: &mut impl TunnelBuilder, rows: u16, cols: u16) -> Tunnel {
        let mut t = Tunnel {
            player: 0,
            screen_width: cols,
            walls: VecDeque::new(),
        };
        t.player = b.choose_player_start(cols);
        for _ in 0..rows_to_loop_iterations(rows) {
            t.add_one_row(b);
        }
        t
    }

    fn add_one_row(&mut self, b: &mut impl TunnelBuilder) {
        self.guarantee_row_precondition();
        let mut new_row = self.walls.back().unwrap().clone();
        if new_row.gap_to_right_wall > 1 {
            new_row.gap_to_right_wall -= 1;
        }
        match b.choose_step() {
            TunnelBuilderChoice::MoveLeftWall => {
                if new_row.left_wall.saturating_add(3) < self.screen_width {
                    new_row.left_wall += 1;
                }
            }
            TunnelBuilderChoice::MoveRightWall => {
                if new_row.gap_to_right_wall == 1 {
                    new_row.left_wall = new_row.left_wall.saturating_sub(1);
                }
            }
        }
        self.walls.push_back(new_row);
    }

    fn guarantee_row_precondition(&mut self) {
        if self.walls.is_empty() {
            self.walls.push_back(Walls {
                left_wall: 0,
                gap_to_right_wall: self.screen_width.saturating_sub(2),
            });
        }
    }

    pub fn move_player_left(&mut self) {
        self.player = self.player.saturating_sub(1);
    }

    pub fn move_player_right(&mut self) {
        self.player = self.player.saturating_add(1);
    }

    pub fn is_collision(&self) -> bool {
        match self.walls.front() {
            Some(wall) => wall.in_wall(self.player),
            None => false,
        }
    }

    pub fn step(&mut self, b: &mut impl TunnelBuilder) {
        self.add_one_row(b);
        self.walls.pop_front();
    }

    pub fn accept(&self, v: &mut impl TunnelCellVisitor) -> Result {
        for (row, wall) in self.walls.iter().enumerate() {
            for column in 0..self.screen_width {
                let cell_type = if row == 0 && column == self.player {
                    TunnelCellType::Player
                } else if wall.in_wall(column) {
                    TunnelCellType::Wall
                } else {
                    TunnelCellType::Floor
                };
                v.visit(row.try_into().unwrap_or(u16::MAX), column, cell_type)?;
            }
        }
        Ok(())
    }
}

pub enum TunnelBuilderChoice {
    MoveLeftWall,
    MoveRightWall,
}

pub trait TunnelBuilder {
    fn choose_player_start(&mut self, max: u16) -> u16;
    fn choose_step(&mut self) -> TunnelBuilderChoice;
}

#[derive(Debug, PartialEq)]
pub enum TunnelCellType {
    Player,
    Floor,
    Wall,
}

pub trait TunnelCellVisitor {
    fn visit(&mut self, x: u16, y: u16, typ: TunnelCellType) -> Result;
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;
    use std::error::Error;
    use std::fmt::{self, Display, Formatter};

    struct MoveWallsPeriodically {
        b: bool,
        count: u16,
        period: u16,
    }
    impl TunnelBuilder for MoveWallsPeriodically {
        fn choose_player_start(&mut self, max: u16) -> u16 {
            if self.b { max.saturating_sub(2) } else { 1 }
        }
        fn choose_step(&mut self) -> TunnelBuilderChoice {
            if self.b && self.count < self.period {
                self.count += 1;
                TunnelBuilderChoice::MoveLeftWall
            } else if !self.b && self.count > 0 {
                self.count -= 1;
                TunnelBuilderChoice::MoveRightWall
            } else {
                self.b = !self.b;
                self.choose_step()
            }
        }
    }

    struct MoveWallsEvenly {
        b: bool,
    }
    impl TunnelBuilder for MoveWallsEvenly {
        fn choose_player_start(&mut self, max: u16) -> u16 {
            max / 2
        }
        fn choose_step(&mut self) -> TunnelBuilderChoice {
            self.b = !self.b;
            if self.b {
                TunnelBuilderChoice::MoveLeftWall
            } else {
                TunnelBuilderChoice::MoveRightWall
            }
        }
    }

    struct GetFirstRow {
        cells: Vec<TunnelCellType>,
    }
    impl TunnelCellVisitor for GetFirstRow {
        fn visit(&mut self, x: u16, _y: u16, typ: TunnelCellType) -> Result {
            if x == 0 {
                self.cells.push(typ);
            }
            Ok(())
        }
    }

    const SIZE: u16 = 5;
    const REPEAT_STEPS: usize = 8;

    #[test]
    fn always_move_left_wall() {
        let mut first_row = GetFirstRow { cells: Vec::new() };

        let mut builder = MoveWallsPeriodically {
            b: true,
            count: 0,
            period: rows_to_loop_iterations(SIZE),
        };
        let mut t = Tunnel::new(&mut builder, SIZE, SIZE);
        assert!(!t.is_collision());

        let expected = vec![
            TunnelCellType::Wall,
            TunnelCellType::Floor,
            TunnelCellType::Floor,
            TunnelCellType::Player,
            TunnelCellType::Wall,
        ];
        t.accept(&mut first_row).unwrap();
        assert_eq!(expected, first_row.cells);
        first_row.cells.clear();

        for _ in 0..rows_to_loop_iterations(SIZE) {
            t.step(&mut builder);
        }
        assert!(!t.is_collision());

        let expected = vec![
            TunnelCellType::Wall,
            TunnelCellType::Wall,
            TunnelCellType::Wall,
            TunnelCellType::Player,
            TunnelCellType::Wall,
        ];
        t.accept(&mut first_row).unwrap();
        assert_eq!(expected, first_row.cells);
        first_row.cells.clear();
    }

    #[test]
    fn always_move_right_wall() {
        let mut first_row = GetFirstRow { cells: Vec::new() };

        let mut builder = MoveWallsPeriodically {
            b: false,
            count: rows_to_loop_iterations(SIZE),
            period: rows_to_loop_iterations(SIZE),
        };
        let mut t = Tunnel::new(&mut builder, SIZE, SIZE);
        assert!(!t.is_collision());

        let expected = vec![
            TunnelCellType::Wall,
            TunnelCellType::Player,
            TunnelCellType::Floor,
            TunnelCellType::Floor,
            TunnelCellType::Wall,
        ];
        t.accept(&mut first_row).unwrap();
        assert_eq!(expected, first_row.cells);
        first_row.cells.clear();

        for _ in 0..rows_to_loop_iterations(SIZE) {
            t.step(&mut builder);
        }
        assert!(!t.is_collision());

        let expected = vec![
            TunnelCellType::Wall,
            TunnelCellType::Player,
            TunnelCellType::Wall,
            TunnelCellType::Wall,
            TunnelCellType::Wall,
        ];
        t.accept(&mut first_row).unwrap();
        assert_eq!(expected, first_row.cells);
        first_row.cells.clear();
    }

    #[test]
    fn continue_steps_down_narrow_tunnel() {
        let mut first_row = GetFirstRow { cells: Vec::new() };

        let mut builder = MoveWallsPeriodically {
            b: true,
            count: 0,
            period: rows_to_loop_iterations(SIZE),
        };
        let mut t = Tunnel::new(&mut builder, SIZE, SIZE);

        for _ in 0..REPEAT_STEPS {
            for _ in 0..rows_to_loop_iterations(SIZE) {
                t.step(&mut builder);
            }
            assert!(!t.is_collision());

            let expected = vec![
                TunnelCellType::Wall,
                TunnelCellType::Wall,
                TunnelCellType::Wall,
                TunnelCellType::Player,
                TunnelCellType::Wall,
            ];
            t.accept(&mut first_row).unwrap();
            assert_eq!(expected, first_row.cells);
            first_row.cells.clear();

            for _ in 0..rows_to_loop_iterations(SIZE) {
                t.step(&mut builder);
            }
            assert!(t.is_collision());

            for _ in 0..rows_to_loop_iterations(SIZE) {
                t.move_player_left();
            }
            assert!(!t.is_collision());

            let expected = vec![
                TunnelCellType::Wall,
                TunnelCellType::Player,
                TunnelCellType::Wall,
                TunnelCellType::Wall,
                TunnelCellType::Wall,
            ];
            t.accept(&mut first_row).unwrap();
            assert_eq!(expected, first_row.cells);
            first_row.cells.clear();

            for _ in 0..rows_to_loop_iterations(SIZE) {
                t.move_player_right();
            }
            assert!(t.is_collision());
        }
    }

    #[test]
    fn edge_case_size_zero_tunnel_no_underflow() {
        let mut first_row = GetFirstRow { cells: Vec::new() };

        let mut builder = MoveWallsEvenly { b: false };
        let mut t = Tunnel::new(&mut builder, 0, 0);
        assert!(!t.is_collision());

        t.accept(&mut first_row).unwrap();
        assert!(first_row.cells.is_empty());

        t.step(&mut builder);
        assert!(t.is_collision());

        t.accept(&mut first_row).unwrap();
        assert!(first_row.cells.is_empty());
    }

    #[test]
    fn edge_case_size_one_tunnel_no_underflow() {
        let mut first_row = GetFirstRow { cells: Vec::new() };

        let mut builder = MoveWallsEvenly { b: true };
        let mut t = Tunnel::new(&mut builder, 1, 1);
        assert!(!t.is_collision());

        t.accept(&mut first_row).unwrap();
        assert!(first_row.cells.is_empty());

        t.step(&mut builder);
        assert!(t.is_collision());

        t.accept(&mut first_row).unwrap();
        assert_eq!(vec![TunnelCellType::Player], first_row.cells);
        first_row.cells.clear();
    }

    #[test]
    fn edge_case_size_two_tunnel_no_underflow() {
        let mut first_row = GetFirstRow { cells: Vec::new() };

        let mut builder = MoveWallsEvenly { b: true };
        let mut t = Tunnel::new(&mut builder, 2, 2);
        assert!(!t.is_collision());

        t.accept(&mut first_row).unwrap();
        assert!(first_row.cells.is_empty());

        t.step(&mut builder);
        assert!(t.is_collision());

        let expected = vec![TunnelCellType::Wall, TunnelCellType::Player];
        t.accept(&mut first_row).unwrap();
        assert_eq!(expected, first_row.cells);
        first_row.cells.clear();
    }

    #[test]
    fn minimum_valid_size_three_tunnel_no_underflow() {
        let mut first_row = GetFirstRow { cells: Vec::new() };

        let mut builder = MoveWallsEvenly { b: true };
        let mut t = Tunnel::new(&mut builder, 3, 3);
        assert!(!t.is_collision());

        t.accept(&mut first_row).unwrap();
        assert!(first_row.cells.is_empty());

        let expected = vec![
            TunnelCellType::Wall,
            TunnelCellType::Player,
            TunnelCellType::Wall,
        ];

        for _ in 0..REPEAT_STEPS {
            t.step(&mut builder);
            assert!(!t.is_collision());

            t.accept(&mut first_row).unwrap();
            assert_eq!(expected, first_row.cells);
            first_row.cells.clear();
        }
    }

    #[derive(Debug)]
    struct VisitorError {}

    impl Display for VisitorError {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            write!(f, "VisitorError for VisitorAlwaysFails")
        }
    }

    impl Error for VisitorError {}

    struct VisitorAlwaysFails {}
    impl TunnelCellVisitor for VisitorAlwaysFails {
        fn visit(&mut self, _x: u16, _y: u16, _typ: TunnelCellType) -> Result {
            Err(Box::new(VisitorError {}))
        }
    }

    #[test]
    fn visitor_result_err() {
        let mut visitor_always_fails = VisitorAlwaysFails {};

        let mut builder = MoveWallsEvenly { b: true };
        let t = Tunnel::new(&mut builder, SIZE, SIZE);

        assert!(t.accept(&mut visitor_always_fails).is_err());
    }
}
