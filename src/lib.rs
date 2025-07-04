// https://github.com/taiki-e/cargo-llvm-cov#exclude-code-from-coverage
#![cfg_attr(coverage_nightly, feature(coverage_attribute))]

use num::iter::Range as NumRange; // clearly distinguish from std::ops::Range
use num::{FromPrimitive, PrimInt, Unsigned, traits::NumAssign};
use std::collections::{VecDeque, vec_deque::Iter as VecDequeIterator};
use std::iter::{Cycle, Peekable, Zip, zip};

pub type Result = std::result::Result<(), Box<dyn std::error::Error>>;

pub trait TunnelIndex:
    From<u8> + FromPrimitive + NumAssign + PrimInt + Unsigned
{
}
impl TunnelIndex for u8 {}
impl TunnelIndex for u16 {}
impl TunnelIndex for u32 {}
impl TunnelIndex for u64 {}
impl TunnelIndex for u128 {}
impl TunnelIndex for usize {}

use num::one;
use num::zero;

fn two<T: TunnelIndex>() -> T {
    2.into()
}

fn three<T: TunnelIndex>() -> T {
    3.into()
}

fn rows_to_loop_iterations<T: TunnelIndex>(rows: T) -> T {
    rows.saturating_sub(three())
}

fn zero_to<T: TunnelIndex>(max: T) -> NumRange<T> {
    num::range(zero(), max)
}

pub struct Tunnel<T: TunnelIndex> {
    player: T,
    screen_width: T,
    walls: VecDeque<TunnelWalls<T>>,
}

impl<T: TunnelIndex> Tunnel<T> {
    pub fn new(b: &mut impl TunnelBuilder, rows: T, cols: T) -> Tunnel<T> {
        let mut t = Tunnel {
            player: zero(),
            screen_width: cols,
            walls: VecDeque::new(),
        };
        t.player = b.choose_player_start(cols);
        for _ in zero_to(rows_to_loop_iterations(rows)) {
            t.add_one_row(b);
        }
        t
    }

    fn add_one_row(&mut self, b: &mut impl TunnelBuilder) {
        let mut new_row = self.clone_last_row();
        if new_row.gap_to_right_wall > one() {
            new_row.gap_to_right_wall -= one();
        }
        match b.choose_step() {
            TunnelBuilderChoice::MoveLeftWall => {
                if new_row.left_wall.saturating_add(three()) < self.screen_width
                {
                    new_row.left_wall += one();
                }
            }
            TunnelBuilderChoice::MoveRightWall => {
                if new_row.gap_to_right_wall == one() {
                    new_row.left_wall = new_row.left_wall.saturating_sub(one());
                }
            }
        }
        self.walls.push_back(new_row);
    }

    fn clone_last_row(&mut self) -> TunnelWalls<T> {
        match self.walls.back() {
            Some(n) => n.clone(),
            None => {
                let new_row = TunnelWalls {
                    left_wall: zero(),
                    gap_to_right_wall: self.screen_width.saturating_sub(two()),
                };
                self.walls.push_back(new_row.clone());
                new_row
            }
        }
    }

    pub fn move_player_left(&mut self) {
        self.player = self.player.saturating_sub(one());
    }

    pub fn move_player_right(&mut self) {
        self.player = self.player.saturating_add(one());
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

    pub fn iter<'a>(&'a self) -> TunnelIterator<'a, T> {
        let w_len = match FromPrimitive::from_usize(self.walls.len()) {
            Some(val) => val,
            None => zero(),
        };
        TunnelIterator {
            player: self.player,
            rows: zip(zero_to(w_len), self.walls.iter()).peekable(),
            cols: zero_to(self.screen_width).cycle().peekable(),
        }
    }
}

pub enum TunnelBuilderChoice {
    MoveLeftWall,
    MoveRightWall,
}

pub trait TunnelBuilder {
    fn choose_player_start<T: TunnelIndex>(&mut self, max: T) -> T;
    fn choose_step(&mut self) -> TunnelBuilderChoice;
}

#[derive(Debug, PartialEq)]
pub enum TunnelCellType {
    Player,
    Floor,
    Wall,
}

type TunnelIteratorItem<T> = (T, T, TunnelCellType);

pub struct TunnelIterator<'a, T: TunnelIndex> {
    player: T,
    rows: Peekable<Zip<NumRange<T>, VecDequeIterator<'a, TunnelWalls<T>>>>,
    cols: Peekable<Cycle<NumRange<T>>>,
}

impl<T: TunnelIndex> Iterator for TunnelIterator<'_, T> {
    type Item = TunnelIteratorItem<T>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.rows.peek() {
            Some(&(row, walls)) => {
                let item = match self.cols.next() {
                    Some(col) => {
                        Some((row, col, walls.cell_type(self.player, row, col)))
                    }
                    None => None, // edge case: zero-size Cycle
                };
                if let Some(next_col) = self.cols.peek()
                    && next_col.is_zero()
                {
                    self.rows.next(); // prepare for next row
                }
                item
            }
            None => None, // consumed all available rows
        }
    }
}

impl<'a, T: TunnelIndex> IntoIterator for &'a Tunnel<T> {
    type Item = TunnelIteratorItem<T>;
    type IntoIter = TunnelIterator<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[derive(Clone)]
struct TunnelWalls<T> {
    left_wall: T,
    gap_to_right_wall: T,
}

impl<T: TunnelIndex> TunnelWalls<T> {
    fn in_wall(&self, column: T) -> bool {
        column <= self.left_wall
            || column > self.left_wall.saturating_add(self.gap_to_right_wall)
    }
    fn cell_type(&self, player: T, row: T, column: T) -> TunnelCellType {
        if row.is_zero() && column == player {
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
    type Idx = u8;

    struct MoveWallsPeriodically {
        b: bool,
        count: Idx,
        period: Idx,
    }
    impl TunnelBuilder for MoveWallsPeriodically {
        fn choose_player_start<T: TunnelIndex>(&mut self, max: T) -> T {
            if self.b {
                max.saturating_sub(two())
            } else {
                one()
            }
        }
        fn choose_step(&mut self) -> TunnelBuilderChoice {
            if self.b && self.count < self.period {
                self.count += &one();
                TunnelBuilderChoice::MoveLeftWall
            } else if !self.b && self.count > zero() {
                self.count -= &one();
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
        fn choose_player_start<T: TunnelIndex>(&mut self, max: T) -> T {
            max / two()
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

    fn get_first_row(t: &Tunnel<Idx>) -> Vec<TunnelCellType> {
        t.iter()
            .filter(|(row, _, _)| *row == zero())
            .map(|(_, _, cell_type)| cell_type)
            .collect()
    }

    const SIZE: Idx = 5;
    const REPEAT_STEPS: Idx = 8;

    #[test]
    fn implicit_loop_into_iterator_vs_explicit_iter_call() {
        let mut builder = MoveWallsEvenly { b: true };
        let t = Tunnel::new(&mut builder, SIZE, SIZE);
        let mut count_into_iterator_loop: usize = zero();
        for _ in &t {
            count_into_iterator_loop += &one();
        }
        assert_eq!(count_into_iterator_loop, t.iter().count());
    }

    #[test]
    fn always_move_left_wall() {
        let mut builder = MoveWallsPeriodically {
            b: true,
            count: zero(),
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

        for _ in zero_to(rows_to_loop_iterations(SIZE)) {
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

        for _ in zero_to(rows_to_loop_iterations(SIZE)) {
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
            count: zero(),
            period: rows_to_loop_iterations(SIZE),
        };
        let mut t = Tunnel::new(&mut builder, SIZE, SIZE);

        for _ in zero_to(REPEAT_STEPS) {
            for _ in zero_to(rows_to_loop_iterations(SIZE)) {
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

            for _ in zero_to(rows_to_loop_iterations(SIZE)) {
                t.step(&mut builder);
            }
            assert!(t.is_collision());

            for _ in zero_to(rows_to_loop_iterations(SIZE)) {
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

            for _ in zero_to(rows_to_loop_iterations(SIZE)) {
                t.move_player_right();
            }
            assert!(t.is_collision());
        }
    }

    #[test]
    fn no_underflow_on_invalid_tunnel_size_zero_rows() {
        let mut builder = MoveWallsEvenly { b: false };
        let mut t = Tunnel::new(&mut builder, zero(), zero());
        assert!(!t.is_collision());
        assert!(get_first_row(&t).is_empty());
        assert_eq!(t.iter().count(), zero());

        t.step(&mut builder);
        assert!(t.is_collision());
        assert!(get_first_row(&t).is_empty());
        assert_eq!(t.iter().count(), zero());
    }

    #[test]
    fn no_underflow_on_invalid_tunnel_size_zero_columns() {
        let mut builder = MoveWallsEvenly { b: false };
        let mut t = Tunnel::new(&mut builder, SIZE, zero());
        assert!(t.is_collision());
        assert!(get_first_row(&t).is_empty());
        assert_eq!(t.iter().count(), zero());

        t.step(&mut builder);
        assert!(t.is_collision());
        assert!(get_first_row(&t).is_empty());
        assert_eq!(t.iter().count(), zero());
    }

    #[test]
    fn no_underflow_on_invalid_tunnel_size_one() {
        let mut builder = MoveWallsEvenly { b: true };
        let mut t = Tunnel::new(&mut builder, one(), one());
        assert!(!t.is_collision());
        assert!(get_first_row(&t).is_empty());

        t.step(&mut builder);
        assert!(t.is_collision());
        assert_eq!(vec![TunnelCellType::Player], get_first_row(&t));
    }

    #[test]
    fn no_underflow_on_invalid_tunnel_size_two() {
        let mut builder = MoveWallsEvenly { b: true };
        let mut t = Tunnel::new(&mut builder, two(), two());
        assert!(!t.is_collision());
        assert!(get_first_row(&t).is_empty());

        t.step(&mut builder);
        assert!(t.is_collision());

        let expected = vec![TunnelCellType::Wall, TunnelCellType::Player];
        assert_eq!(expected, get_first_row(&t));
    }

    #[test]
    fn no_underflow_on_small_yet_valid_tunnel_size_three() {
        let mut builder = MoveWallsEvenly { b: true };
        let mut t = Tunnel::new(&mut builder, three(), three());
        assert!(!t.is_collision());
        assert!(get_first_row(&t).is_empty());

        let expected = vec![
            TunnelCellType::Wall,
            TunnelCellType::Player,
            TunnelCellType::Wall,
        ];

        for _ in zero_to(REPEAT_STEPS) {
            t.step(&mut builder);
            assert!(!t.is_collision());
            assert_eq!(expected, get_first_row(&t));
        }
    }

    #[test]
    fn no_overflow_on_tunnel_size_greater_than_u8_max() {
        let mut builder = MoveWallsEvenly { b: true };
        // create rows and columns that barely fit into Tunnel<u8>
        let mut t = Tunnel::<u8>::new(&mut builder, u8::MAX, u8::MAX);
        // check precondition: Tunnel initially looks reasonable
        assert!(t.iter().next().is_some());
        let u8_max_as_usize = Into::<usize>::into(u8::MAX);
        assert_eq!(t.iter().count() / u8_max_as_usize, (u8::MAX - 2).into());
        // use private APIs to cause inconsistency: number_of_rows > u8::MAX
        for _ in zero_to::<u8>(three()) {
            t.add_one_row(&mut builder);
        }
        // exercise FromPrimitive::from_usize() overflowing narrower u8 value,
        // resulting in empty-looking iter() that at least avoids crashing
        assert!(t.iter().next().is_none());
    }
}
