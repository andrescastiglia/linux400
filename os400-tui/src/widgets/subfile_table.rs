use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    widgets::{Block, Borders, Row, Table, TableState},
};

use crate::style::*;

/// A reusable paginated table widget that replaces the duplicated
/// `Table`+`TableState` pattern found in ObjectBrowser, WorkManagement,
/// SpoolOutq and WrkMbrPdm.
///
/// Features:
/// - Column headers with configurable widths
/// - Row-level numeric options (like 5250 subfile)
/// - Selection highlight
/// - Scroll with page up/down
/// - Auto-selects first row when non-empty
pub struct SubfileTable {
    headers: Vec<String>,
    widths: Vec<u16>,
    rows: Vec<Vec<String>>,
    state: TableState,
    title: Option<String>,
}

impl SubfileTable {
    pub fn new(headers: Vec<impl Into<String>>, widths: Vec<u16>) -> Self {
        Self {
            headers: headers.into_iter().map(Into::into).collect(),
            widths,
            rows: Vec::new(),
            state: TableState::default(),
            title: None,
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Replace all rows. Auto-selects the first row if available.
    pub fn set_rows(&mut self, rows: Vec<Vec<String>>) {
        self.rows = rows;
        if self.rows.is_empty() {
            self.state.select(None);
        } else if self.state.selected().is_none() {
            self.state.select(Some(0));
        } else if let Some(index) = self.state.selected()
            && index >= self.rows.len()
        {
            self.state.select(Some(self.rows.len().saturating_sub(1)));
        }
    }

    /// Get the currently selected row index.
    pub fn selected(&self) -> Option<usize> {
        self.state.selected()
    }

    /// Get the data of the currently selected row.
    pub fn selected_row(&self) -> Option<&Vec<String>> {
        self.state.selected().and_then(|index| self.rows.get(index))
    }

    /// Move selection up by one row.
    pub fn select_prev(&mut self) {
        if let Some(current) = self.state.selected() {
            self.state.select(Some(current.saturating_sub(1)));
        }
    }

    /// Move selection down by one row.
    pub fn select_next(&mut self) {
        if let Some(current) = self.state.selected() {
            let max = self.rows.len().saturating_sub(1);
            self.state.select(Some(current.saturating_add(1).min(max)));
        }
    }

    /// Move selection up by a page.
    pub fn page_up(&mut self) {
        if let Some(current) = self.state.selected() {
            self.state.select(Some(current.saturating_sub(10)));
        }
    }

    /// Move selection down by a page.
    pub fn page_down(&mut self) {
        if let Some(current) = self.state.selected() {
            let max = self.rows.len().saturating_sub(1);
            self.state.select(Some(current.saturating_add(10).min(max)));
        }
    }

    /// Total number of rows.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Whether the table is empty.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Render the subfile table into the given area.
    pub fn render(&mut self, frame: &mut Frame, area: Rect) {
        let ratatui_rows: Vec<Row> = self.rows.iter().map(|row| Row::new(row.clone())).collect();

        let constraints: Vec<Constraint> =
            self.widths.iter().map(|w| Constraint::Length(*w)).collect();

        let mut block = Block::default()
            .borders(Borders::ALL)
            .border_style(STYLE_BORDER);

        if let Some(title) = &self.title {
            block = block.title(format!(" {} ", title));
        }

        let table = Table::new(ratatui_rows, constraints)
            .header(
                Row::new(self.headers.clone())
                    .style(STYLE_TABLE_HEADER)
                    .height(1),
            )
            .block(block)
            .style(STYLE_NORMAL)
            .row_highlight_style(STYLE_SELECTION);

        frame.render_stateful_widget(table, area, &mut self.state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_selects_first_row() {
        let mut table = SubfileTable::new(vec!["A", "B"], vec![10, 10]);
        table.set_rows(vec![vec!["1".to_string(), "2".to_string()]]);
        assert_eq!(table.selected(), Some(0));
    }

    #[test]
    fn empty_table_has_no_selection() {
        let mut table = SubfileTable::new(vec!["A"], vec![10]);
        table.set_rows(vec![]);
        assert_eq!(table.selected(), None);
    }

    #[test]
    fn navigation_clamps_to_bounds() {
        let mut table = SubfileTable::new(vec!["A"], vec![10]);
        table.set_rows(vec![
            vec!["a".to_string()],
            vec!["b".to_string()],
            vec!["c".to_string()],
        ]);

        table.select_prev(); // already at 0
        assert_eq!(table.selected(), Some(0));

        table.select_next();
        table.select_next();
        assert_eq!(table.selected(), Some(2));

        table.select_next(); // clamps at 2
        assert_eq!(table.selected(), Some(2));
    }

    #[test]
    fn page_navigation() {
        let mut table = SubfileTable::new(vec!["A"], vec![10]);
        let rows: Vec<Vec<String>> = (0..25).map(|i| vec![i.to_string()]).collect();
        table.set_rows(rows);

        table.page_down();
        assert_eq!(table.selected(), Some(10));

        table.page_down();
        assert_eq!(table.selected(), Some(20));

        table.page_down(); // clamps to 24
        assert_eq!(table.selected(), Some(24));

        table.page_up();
        assert_eq!(table.selected(), Some(14));
    }

    #[test]
    fn selection_clamps_when_rows_shrink() {
        let mut table = SubfileTable::new(vec!["A"], vec![10]);
        table.set_rows(vec![vec!["a".to_string()], vec!["b".to_string()]]);
        table.select_next(); // at index 1
        assert_eq!(table.selected(), Some(1));

        table.set_rows(vec![vec!["a".to_string()]]); // shrink
        assert_eq!(table.selected(), Some(0)); // clamped
    }
}
