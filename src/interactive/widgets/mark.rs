use crate::interactive::{
    fit_string_graphemes_with_ellipsis, widgets::COLOR_MARKED_LIGHT, CursorDirection,
};
use dua::{
    path_of,
    traverse::{Tree, TreeIndex},
    ByteFormat,
};
use itertools::Itertools;
use std::{borrow::Borrow, collections::btree_map::Entry, collections::BTreeMap, path::PathBuf};
use termion::{event::Key, event::Key::*};
use tui::{
    buffer::Buffer,
    layout::Rect,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::Block,
    widgets::Borders,
    widgets::Text,
    widgets::{Paragraph, Widget},
};
use tui_react::{List, ListProps};
use unicode_segmentation::UnicodeSegmentation;

pub type EntryMarkMap = BTreeMap<TreeIndex, EntryMark>;
pub struct EntryMark {
    pub size: u64,
    pub path: PathBuf,
    pub index: usize,
}

#[derive(Default)]
pub struct MarkPane {
    selected: Option<usize>,
    marked: EntryMarkMap,
    list: List,
    has_focus: bool,
    last_sorting_index: usize,
}

pub struct MarkPaneProps {
    pub border_style: Style,
    pub format: ByteFormat,
}

impl MarkPane {
    #[cfg(test)]
    pub fn has_focus(&self) -> bool {
        self.has_focus
    }
    pub fn set_focus(&mut self, has_focus: bool) {
        self.has_focus = has_focus;
        if has_focus {
            self.selected = Some(self.marked.len().saturating_sub(1));
        } else {
            self.selected = None
        }
    }
    pub fn toggle_index(mut self, index: TreeIndex, tree: &Tree) -> Option<Self> {
        match self.marked.entry(index) {
            Entry::Vacant(entry) => {
                if let Some(e) = tree.node_weight(index) {
                    let sorting_index = self.last_sorting_index + 1;
                    self.last_sorting_index = sorting_index;
                    entry.insert(EntryMark {
                        size: e.size,
                        path: path_of(tree, index),
                        index: sorting_index,
                    });
                }
            }
            Entry::Occupied(entry) => {
                entry.remove();
            }
        };
        if self.marked.is_empty() {
            None
        } else {
            Some(self)
        }
    }
    pub fn marked(&self) -> &EntryMarkMap {
        &self.marked
    }
    pub fn key(&mut self, key: Key) {
        match key {
            Ctrl('u') | PageUp => self.change_selection(CursorDirection::PageUp),
            Char('k') | Up => self.change_selection(CursorDirection::Up),
            Char('j') | Down => self.change_selection(CursorDirection::Down),
            Ctrl('d') | PageDown => self.change_selection(CursorDirection::PageDown),
            _ => {}
        };
    }

    fn change_selection(&mut self, direction: CursorDirection) {
        self.selected = self.selected.map(|selected| {
            direction
                .move_cursor(selected)
                .min(self.marked.len().saturating_sub(1))
        });
    }

    pub fn render(&mut self, props: impl Borrow<MarkPaneProps>, area: Rect, buf: &mut Buffer) {
        let MarkPaneProps {
            border_style,
            format,
        } = props.borrow();

        let marked: &_ = &self.marked;
        let title = format!(
            "Marked {} items ({}) ",
            marked.len(),
            format.display(marked.iter().map(|(_k, v)| v.size).sum::<u64>())
        );
        let selected = self.selected;
        let entries = marked
            .values()
            .sorted_by_key(|v| &v.index)
            .enumerate()
            .map(|(idx, v)| {
                let modifier = match selected {
                    Some(selected) if idx == selected => Modifier::BOLD,
                    _ => Modifier::empty(),
                };
                let (path, path_len) = {
                    let path = format!(" {}  ", v.path.display());
                    let num_path_graphemes = path.graphemes(true).count();
                    match num_path_graphemes + format.total_width() {
                        n if n > area.width as usize => {
                            let desired_size = num_path_graphemes - (n - area.width as usize);
                            fit_string_graphemes_with_ellipsis(
                                path,
                                num_path_graphemes,
                                desired_size,
                            )
                        }
                        _ => (path, num_path_graphemes),
                    }
                };
                let path = Text::Styled(
                    path.into(),
                    Style {
                        fg: COLOR_MARKED_LIGHT,
                        modifier,
                        ..Style::default()
                    },
                );
                let bytes = Text::Styled(
                    format!(
                        "{:>byte_column_width$}",
                        format.display(v.size).to_string(), // we would have to impl alignment/padding ourselves otherwise...
                        byte_column_width = format.width()
                    )
                    .into(),
                    Style {
                        fg: Color::Green,
                        ..Default::default()
                    },
                );
                let spacer = Text::Raw(
                    format!(
                        "{:-space$}",
                        "",
                        space = (area.width as usize)
                            .saturating_sub(path_len)
                            .saturating_sub(format.total_width())
                    )
                    .into(),
                );
                vec![path, spacer, bytes]
            });

        let entry_in_view = match self.selected {
            Some(s) => Some(s),
            None => {
                self.list.offset = 0;
                Some(marked.len().saturating_sub(1))
            }
        };
        let mut block = Block::default()
            .title(&title)
            .border_style(*border_style)
            .borders(Borders::ALL);

        let inner_area = block.inner(area);
        block.draw(area, buf);

        let list_area = if self.has_focus {
            let (help_line_area, list_area) = {
                let regions = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(1), Constraint::Max(256)].as_ref())
                    .split(inner_area);
                (regions[0], regions[1])
            };

            let default_style = Style {
                fg: Color::Black,
                bg: Color::White,
                modifier: Modifier::BOLD,
                ..Default::default()
            };
            Paragraph::new(
                [
                    Text::Styled(
                        " Ctrl + Shift + r".into(),
                        Style {
                            fg: Color::Red,
                            modifier: default_style.modifier | Modifier::RAPID_BLINK,
                            ..default_style
                        },
                    ),
                    Text::Styled(
                        " permanently deletes list without prompt".into(),
                        default_style,
                    ),
                ]
                .iter(),
            )
            .style(Style {
                bg: Color::White,
                ..Style::default()
            })
            .draw(help_line_area, buf);
            list_area
        } else {
            inner_area
        };

        let props = ListProps {
            block: None,
            entry_in_view,
        };
        self.list.render(props, entries, list_area, buf)
    }
}
