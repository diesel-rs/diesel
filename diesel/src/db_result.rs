use connection::Connection;
use row::DbRow;

pub trait DbResult: Drop + Sized {

    type Connection: Connection;

    fn rows_affected(&self) -> usize; 

    fn num_rows(&self) -> usize; 

    fn get_row(&self, idx: usize) -> DbRow<Self>; 

    fn get(&self, row_idx: usize, col_idx: usize) -> Option<&[u8]>; 

    fn is_null(&self, row_idx: usize, col_idx: usize) -> bool; 
}
