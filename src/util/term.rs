use std::io::Write;

use crossterm::{
    ExecutableCommand, cursor,
    terminal::{Clear, ClearType},
};

use crate::prelude::*;

pub fn clean_up_rows(output: &mut impl Write, rows: u16) -> Result<()> {
    output.execute(cursor::MoveUp(rows)).context(TermCtx)?;
    output
        .execute(Clear(ClearType::FromCursorDown))
        .context(TermCtx)?;
    Ok(())
}
