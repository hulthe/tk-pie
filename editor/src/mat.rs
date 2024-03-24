use serde::{Deserialize, Serialize};

/// 2-dimensional dynamically sized matrix
#[derive(Default, Deserialize, Serialize)]
pub struct Mat<T> {
    rows: Vec<Vec<T>>,
}

#[allow(dead_code)]
impl<T> Mat<T> {
    /// Iterater over entries, rows first.
    pub fn iter_rf(&self) -> impl Iterator<Item = &T> {
        todo!();
        #[allow(unreachable_code)]
        [].into_iter()
    }

    /// Iterater over entries, columns first.
    pub fn iter_cf(&self) -> impl Iterator<Item = &T> {
        let height = self.height();
        (0..self.width())
            .flat_map(move |x| (0..height).map(move |y| (x, y)))
            .map(|(x, y)| &self.rows[y][x])
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        if x >= self.width() || y >= self.height() {
            return None;
        }

        Some(&mut self.rows[y][x])
    }

    /// Get the width & height of the matrix
    pub fn height(&self) -> usize {
        self.rows.len()
    }

    /// Get the width & height of the matrix
    pub fn width(&self) -> usize {
        self.rows.get(0).map(|row| row.len()).unwrap_or(0)
    }

    /// Get the width & height of the matrix
    pub fn size(&self) -> (usize, usize) {
        (self.width(), self.height())
    }

    pub fn push_row(&mut self, t: T)
    where
        T: Clone,
    {
        let width = self.width();
        if width == 0 {
            self.rows.push(vec![t]);
        } else {
            self.rows.push(vec![t; width])
        }
    }

    pub fn push_col(&mut self, t: T)
    where
        T: Clone,
    {
        let height = self.height();
        if height == 0 {
            self.rows.push(vec![t]);
        } else {
            self.rows.iter_mut().for_each(|row| row.push(t.clone()));
        }
    }

    pub fn remove_row(&mut self, y: usize) {
        if y >= self.height() {
            panic!("row {y} out of matrix bounds");
        }

        if self.height() == 1 {
            self.rows.clear();
        } else {
            self.rows.remove(y);
        }
    }

    pub fn remove_col(&mut self, x: usize) {
        if x >= self.width() {
            panic!("col {x} out of matrix bounds");
        }

        if self.width() == 1 {
            self.rows.clear();
        } else {
            for row in self.rows.iter_mut() {
                row.remove(x);
            }
        }
    }
}
