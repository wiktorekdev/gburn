use super::app::View;

#[derive(Debug, Clone, Copy)]
pub struct RectHit {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl RectHit {
    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, col: u16, row: u16) -> bool {
        col >= self.x
            && col < self.x.saturating_add(self.width)
            && row >= self.y
            && row < self.y.saturating_add(self.height)
    }

    pub fn from_ratatui(r: ratatui::layout::Rect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct TabHit {
    pub hit: RectHit,
    pub view: View,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipAction {
    Sort,
    TimeRange,
    Search,
}

#[derive(Debug, Clone, Copy)]
pub struct ChipHit {
    pub hit: RectHit,
    pub action: ChipAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverallAction {
    /// cycle time range
    Range,
    /// open sessions list
    Sessions,
    /// open models view
    Models,
    /// open burner session by index into top list
    Burner(usize),
}

#[derive(Debug, Clone, Copy)]
pub struct OverallHit {
    pub hit: RectHit,
    pub action: OverallAction,
}

#[derive(Debug, Default)]
pub struct HitMap {
    pub tabs: Vec<TabHit>,
    pub chips: Vec<ChipHit>,
    pub list: Option<RectHit>,
    pub page_body: Option<RectHit>,
    pub footer_meta: Option<RectHit>,
    pub overall: Vec<OverallHit>,
}

impl HitMap {
    pub fn clear(&mut self) {
        self.tabs.clear();
        self.chips.clear();
        self.list = None;
        self.page_body = None;
        self.footer_meta = None;
        self.overall.clear();
    }

    pub fn tab_at(&self, col: u16, row: u16) -> Option<View> {
        self.tabs
            .iter()
            .find(|t| t.hit.contains(col, row))
            .map(|t| t.view)
    }

    pub fn chip_at(&self, col: u16, row: u16) -> Option<ChipAction> {
        self.chips
            .iter()
            .find(|c| c.hit.contains(col, row))
            .map(|c| c.action)
    }

    pub fn overall_at(&self, col: u16, row: u16) -> Option<OverallAction> {
        self.overall
            .iter()
            .find(|h| h.hit.contains(col, row))
            .map(|h| h.action)
    }

    pub fn list_index_at(&self, col: u16, row: u16, list_scroll: usize) -> Option<usize> {
        let list = self.list?;
        if !list.contains(col, row) {
            return None;
        }
        let rel = row.saturating_sub(list.y) as usize;
        Some(list_scroll + rel)
    }
}
