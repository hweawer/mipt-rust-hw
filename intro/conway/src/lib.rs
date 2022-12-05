#![forbid(unsafe_code)]

////////////////////////////////////////////////////////////////////////////////

const DIMENSIONS: [(i32, i32); 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

pub struct SimpleIter<'a, T> {
    cur_row: usize,
    cur_col: usize,
    grid: &'a Grid<T>,
    dim_idx: usize,
}

impl<'a, T: Clone + Default> SimpleIter<'a, T> {
    fn new(x: usize, y: usize, grid: &'a Grid<T>) -> SimpleIter<'a, T> {
        Self {
            cur_row: x,
            cur_col: y,
            grid,
            dim_idx: 0,
        }
    }

    fn is_valid_location(&self, row: i32, col: i32) -> bool {
        row >= 0 && row < self.grid.rows as i32 && col >= 0 && col < self.grid.cols as i32
    }
}

impl<'a, T: Clone + Default> Iterator for SimpleIter<'a, T> {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        while self.dim_idx < DIMENSIONS.len() {
            let (step_row, step_col) = DIMENSIONS[self.dim_idx];
            self.dim_idx += 1;
            let new_row = self.cur_row as i32 + step_row;
            let new_col = self.cur_col as i32 + step_col;
            if self.is_valid_location(new_row, new_col) {
                return Some((new_row as usize, new_col as usize));
            }
        }
        None
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Grid<T> {
    rows: usize,
    cols: usize,
    grid: Vec<T>,
}

impl<T: Clone + Default> Grid<T> {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            grid: Vec::with_capacity(rows + cols),
        }
    }

    pub fn from_slice(grid: &[T], rows: usize, cols: usize) -> Self {
        Self {
            rows,
            cols,
            grid: grid.to_vec(),
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }

    pub fn get(&self, row: usize, col: usize) -> &T {
        &self.grid[row * self.cols + col]
    }

    pub fn set(&mut self, value: T, row: usize, col: usize) {
        self.grid[row * self.cols + col] = value
    }

    pub fn neighbours(&self, row: usize, col: usize) -> SimpleIter<'_, T> {
        SimpleIter::new(row, col, self)
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    Dead,
    Alive,
}

impl Default for Cell {
    fn default() -> Self {
        Self::Dead
    }
}

////////////////////////////////////////////////////////////////////////////////

#[derive(PartialEq, Eq)]
pub struct GameOfLife {
    grid: Grid<Cell>,
}

impl GameOfLife {
    pub fn from_grid(grid: Grid<Cell>) -> Self {
        Self { grid }
    }

    pub fn get_grid(&self) -> &Grid<Cell> {
        &self.grid
    }

    pub fn step(&mut self) {
        let mut new_state = self.grid.clone();
        for i in 0..self.grid.rows {
            for j in 0..self.grid.cols {
                let cur = self.grid.get(i, j);
                let alive_neighbours = self
                    .grid
                    .neighbours(i, j)
                    .map(|(x, y)| *self.grid.get(x, y))
                    .filter(|cell| *cell == Cell::Alive)
                    .count();
                match cur {
                    Cell::Dead => {
                        if alive_neighbours == 3 {
                            new_state.set(Cell::Alive, i, j)
                        } else {
                            new_state.set(Cell::Dead, i, j)
                        }
                    }
                    Cell::Alive => {
                        if alive_neighbours < 2 || alive_neighbours > 3 {
                            new_state.set(Cell::Dead, i, j)
                        } else {
                            new_state.set(Cell::Alive, i, j)
                        }
                    }
                }
            }
        }
        self.grid = new_state;
    }
}

#[cfg(test)]
mod test {
    use super::{Cell, GameOfLife, Grid};

    fn get_grid(grid: Vec<Vec<u8>>) -> Grid<Cell> {
        let rows = grid.len();
        let cols = grid[0].len();
        let grid: Vec<Cell> = grid
            .into_iter()
            .flatten()
            .map(|value| if value == 0 { Cell::Dead } else { Cell::Alive })
            .collect();
        assert_eq!(grid.len(), rows * cols);
        Grid::from_slice(grid.as_slice(), rows, cols)
    }

    #[test]
    fn grid_neighbours() {
        assert_eq!(
            Grid::<i32>::new(3, 3)
                .neighbours(2, 2)
                .into_iter()
                .collect::<Vec<_>>(),
            vec![(1, 1), (1, 2), (2, 1)]
        );
        assert_eq!(
            Grid::<i32>::new(1, 1)
                .neighbours(0, 0)
                .into_iter()
                .collect::<Vec<_>>(),
            vec![]
        );
        assert_eq!(
            Grid::<i32>::new(3, 4)
                .neighbours(1, 1)
                .into_iter()
                .collect::<Vec<_>>(),
            vec![
                (0, 0),
                (0, 1),
                (0, 2),
                (1, 0),
                (1, 2),
                (2, 0),
                (2, 1),
                (2, 2),
            ]
        );
    }

    #[test]
    fn first_rule() {
        #[rustfmt::skip]
            let grid = get_grid(vec![
            vec![1, 0, 0],
            vec![0, 1, 0],
            vec![0, 0, 0],
        ]);
        let final_grid = get_grid(vec![vec![0, 0, 0], vec![0, 0, 0], vec![0, 0, 0]]);
        let mut game = GameOfLife::from_grid(grid.clone());
        game.step();
        assert!(game.get_grid() == &final_grid);
    }

    #[test]
    fn second_rule() {
        #[rustfmt::skip]
            let grid = get_grid(vec![
            vec![1, 0, 0],
            vec![0, 1, 0],
            vec![0, 0, 1],
        ]);
        #[rustfmt::skip]
            let final_grid = get_grid(vec![
            vec![0, 0, 0],
            vec![0, 1, 0],
            vec![0, 0, 0],
        ]);
        let mut game = GameOfLife::from_grid(grid.clone());
        game.step();
        assert!(game.get_grid() == &final_grid);
    }

    #[test]
    fn third_rule() {
        #[rustfmt::skip]
            let grid = get_grid(vec![
            vec![0, 1, 0],
            vec![1, 1, 1],
            vec![0, 1, 0],
        ]);
        let final_grid = get_grid(vec![vec![1, 1, 1], vec![1, 0, 1], vec![1, 1, 1]]);
        let mut game = GameOfLife::from_grid(grid.clone());
        game.step();
        assert!(game.get_grid() == &final_grid);
    }

    #[test]
    fn fourth_rule() {
        #[rustfmt::skip]
            let grid = get_grid(vec![
            vec![0, 0, 0],
            vec![0, 1, 0],
            vec![1, 0, 1],
        ]);
        #[rustfmt::skip]
            let final_grid = get_grid(vec![
            vec![0, 0, 0],
            vec![0, 1, 0],
            vec![0, 1, 0],
        ]);
        let mut game = GameOfLife::from_grid(grid.clone());
        game.step();
        assert!(game.get_grid() == &final_grid);
    }

    #[test]
    fn glider() {
        let grid1 = get_grid(vec![
            vec![0, 1, 0, 0, 0, 0],
            vec![0, 0, 1, 0, 0, 0],
            vec![1, 1, 1, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 1, 1],
            vec![0, 0, 0, 0, 1, 1],
        ]);
        let grid2 = get_grid(vec![
            vec![0, 0, 0, 0, 0, 0],
            vec![1, 0, 1, 0, 0, 0],
            vec![0, 1, 1, 0, 0, 0],
            vec![0, 1, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 1, 1],
            vec![0, 0, 0, 0, 1, 1],
        ]);
        let grid3 = get_grid(vec![
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 0, 1, 0, 0, 0],
            vec![1, 0, 1, 0, 0, 0],
            vec![0, 1, 1, 0, 0, 0],
            vec![0, 0, 0, 0, 1, 1],
            vec![0, 0, 0, 0, 1, 1],
        ]);
        let grid4 = get_grid(vec![
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 1, 0, 0, 0, 0],
            vec![0, 0, 1, 1, 0, 0],
            vec![0, 1, 1, 1, 0, 0],
            vec![0, 0, 0, 1, 1, 1],
            vec![0, 0, 0, 0, 1, 1],
        ]);
        let grid5 = get_grid(vec![
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 0, 1, 0, 0, 0],
            vec![0, 0, 0, 1, 0, 0],
            vec![0, 1, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 1],
            vec![0, 0, 0, 1, 0, 1],
        ]);
        let grid6 = get_grid(vec![
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 0, 1, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 1, 0],
            vec![0, 0, 0, 0, 1, 0],
        ]);
        let grid7 = get_grid(vec![
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0],
            vec![0, 0, 0, 0, 0, 0],
        ]);

        let mut game = GameOfLife::from_grid(grid1.clone());
        assert!(game.get_grid() == &grid1);
        game.step();
        assert!(game.get_grid() == &grid2);
        game.step();
        assert!(game.get_grid() == &grid3);
        game.step();
        assert!(game.get_grid() == &grid4);
        game.step();
        assert!(game.get_grid() == &grid5);
        game.step();
        assert!(game.get_grid() == &grid6);
        game.step();
        assert!(game.get_grid() == &grid7);
        game.step();
        assert!(game.get_grid() == &grid7);
    }
}
