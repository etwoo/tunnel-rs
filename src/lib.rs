// https://github.com/taiki-e/cargo-llvm-cov#exclude-code-from-coverage
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use core::ops::Range;
use std::collections::{VecDeque, vec_deque};
use std::iter::{Cycle, Enumerate, Peekable};

pub type Result = std::result::Result<(), Box<dyn std::error::Error>>;

fn rows_to_loop_iterations(rows: u16) -> u16 {
    rows.saturating_sub(3)
}

pub struct Tunnel {
    player: u16,
    screen_width: u16,
    walls: VecDeque<TunnelWalls>,
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
        let mut new_row = *self.walls.back().unwrap();
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
            self.walls.push_back(TunnelWalls {
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

    pub fn iter<'a>(&'a self) -> TunnelIterator<'a> {
        TunnelIterator {
            player: self.player,
            rows: self.walls.iter().enumerate().peekable(),
            cols: (0..self.screen_width).cycle().peekable(),
        }
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

type TunnelIteratorItem = (u16, u16, TunnelCellType);

pub struct TunnelIterator<'a> {
    player: u16,
    rows: Peekable<Enumerate<vec_deque::Iter<'a, TunnelWalls>>>,
    cols: Peekable<Cycle<Range<u16>>>,
}

impl TunnelIterator<'_> {
    fn choose(&mut self) -> Option<TunnelIteratorItem> {
        match self.rows.peek() {
            Some(&(row_as_usize, walls)) => {
                let item = match self.cols.next() {
                    Some(col) => {
                        let row = row_as_usize.try_into().unwrap_or(u16::MAX);
                        Some((row, col, walls.cell_type(self.player, row, col)))
                    }
                    None => None, // edge case: zero-size Cycle
                };
                if let Some(0) = self.cols.peek() {
                    self.rows.next(); // prepare for next row
                }
                item
            }
            None => None, // consumed all available rows
        }
    }
}

impl Iterator for TunnelIterator<'_> {
    type Item = TunnelIteratorItem;
    fn next(&mut self) -> Option<Self::Item> {
        self.choose()
    }
}

#[derive(Clone, Copy)]
struct TunnelWalls {
    left_wall: u16,
    gap_to_right_wall: u16,
}

impl TunnelWalls {
    fn in_wall(&self, column: u16) -> bool {
        column <= self.left_wall
            || column > self.left_wall + self.gap_to_right_wall
    }
    fn cell_type(&self, player: u16, row: u16, column: u16) -> TunnelCellType {
        if row == 0 && column == player {
            TunnelCellType::Player
        } else if self.in_wall(column) {
            TunnelCellType::Wall
        } else {
            TunnelCellType::Floor
        }
    }
}

#[cfg(test)]
#[cfg_attr(coverage_nightly, coverage(off))]
mod tests {
    use super::*;

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

    fn get_first_row(t: &Tunnel) -> Vec<TunnelCellType> {
        t.iter()
            .filter(|(row, _, _)| *row == 0)
            .map(|(_, _, cell_type)| cell_type)
            .collect()
    }

    const SIZE: u16 = 5;
    const REPEAT_STEPS: usize = 8;

    #[test]
    fn always_move_left_wall() {
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
        assert_eq!(expected, get_first_row(&t));

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
        assert_eq!(expected, get_first_row(&t));
    }

    #[test]
    fn always_move_right_wall() {
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
        assert_eq!(expected, get_first_row(&t));

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
        assert_eq!(expected, get_first_row(&t));
    }

    #[test]
    fn continue_steps_down_narrow_tunnel() {
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
            assert_eq!(expected, get_first_row(&t));

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
            assert_eq!(expected, get_first_row(&t));

            for _ in 0..rows_to_loop_iterations(SIZE) {
                t.move_player_right();
            }
            assert!(t.is_collision());
        }
    }

    #[test]
    fn edge_case_size_zero_tunnel_no_underflow() {
        let mut builder = MoveWallsEvenly { b: false };
        let mut t = Tunnel::new(&mut builder, 0, 0);
        assert!(!t.is_collision());
        assert!(get_first_row(&t).is_empty());

        t.step(&mut builder);
        assert!(t.is_collision());
        assert!(get_first_row(&t).is_empty());
    }

    #[test]
    fn edge_case_size_one_tunnel_no_underflow() {
        let mut builder = MoveWallsEvenly { b: true };
        let mut t = Tunnel::new(&mut builder, 1, 1);
        assert!(!t.is_collision());
        assert!(get_first_row(&t).is_empty());

        t.step(&mut builder);
        assert!(t.is_collision());
        assert_eq!(vec![TunnelCellType::Player], get_first_row(&t));
    }

    #[test]
    fn edge_case_size_two_tunnel_no_underflow() {
        let mut builder = MoveWallsEvenly { b: true };
        let mut t = Tunnel::new(&mut builder, 2, 2);
        assert!(!t.is_collision());
        assert!(get_first_row(&t).is_empty());

        t.step(&mut builder);
        assert!(t.is_collision());

        let expected = vec![TunnelCellType::Wall, TunnelCellType::Player];
        assert_eq!(expected, get_first_row(&t));
    }

    #[test]
    fn minimum_valid_size_three_tunnel_no_underflow() {
        let mut builder = MoveWallsEvenly { b: true };
        let mut t = Tunnel::new(&mut builder, 3, 3);
        assert!(!t.is_collision());
        assert!(get_first_row(&t).is_empty());

        let expected = vec![
            TunnelCellType::Wall,
            TunnelCellType::Player,
            TunnelCellType::Wall,
        ];

        for _ in 0..REPEAT_STEPS {
            t.step(&mut builder);
            assert!(!t.is_collision());
            assert_eq!(expected, get_first_row(&t));
        }
    }
}
