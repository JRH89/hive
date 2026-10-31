//! Tabs of split panes. Each pane shows one session; a session is shown in
//! at most one pane. Pure geometry, so the app and the renderer agree on
//! where every pane is.

use hive_protocol::SessionId;
use ratatui::layout::Rect;

/// Where a split puts the new pane.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    /// Side by side; the new pane on the right.
    Right,
    /// Stacked; the new pane below.
    Down,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    Pane(SessionId),
    /// Two halves, `a` left/top and `b` right/bottom.
    Split {
        dir: Dir,
        a: Box<Node>,
        b: Box<Node>,
    },
}

/// One pane on screen: a title row (empty when there's only one pane) and
/// the terminal area below it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PaneArea {
    pub session: SessionId,
    pub title: Rect,
    pub body: Rect,
}

impl Node {
    pub fn panes(&self) -> Vec<SessionId> {
        match self {
            Node::Pane(id) => vec![*id],
            Node::Split { a, b, .. } => {
                let mut out = a.panes();
                out.extend(b.panes());
                out
            }
        }
    }

    pub fn contains(&self, id: SessionId) -> bool {
        match self {
            Node::Pane(p) => *p == id,
            Node::Split { a, b, .. } => a.contains(id) || b.contains(id),
        }
    }

    fn replace(&mut self, old: SessionId, new: SessionId) -> bool {
        match self {
            Node::Pane(p) if *p == old => {
                *p = new;
                true
            }
            Node::Pane(_) => false,
            Node::Split { a, b, .. } => a.replace(old, new) || b.replace(old, new),
        }
    }

    fn split(&mut self, target: SessionId, new: SessionId, dir: Dir) -> bool {
        match self {
            Node::Pane(p) if *p == target => {
                *self = Node::Split {
                    dir,
                    a: Box::new(Node::Pane(target)),
                    b: Box::new(Node::Pane(new)),
                };
                true
            }
            Node::Pane(_) => false,
            Node::Split { a, b, .. } => a.split(target, new, dir) || b.split(target, new, dir),
        }
    }

    /// The tree without `id`; its sibling takes the freed space. `None` if
    /// nothing is left.
    fn without(self, id: SessionId) -> Option<Node> {
        match self {
            Node::Pane(p) if p == id => None,
            pane @ Node::Pane(_) => Some(pane),
            Node::Split { dir, a, b } => match (a.without(id), b.without(id)) {
                (Some(a), Some(b)) => Some(Node::Split {
                    dir,
                    a: Box::new(a),
                    b: Box::new(b),
                }),
                (Some(n), None) | (None, Some(n)) => Some(n),
                (None, None) => None,
            },
        }
    }

    /// Splits `area` among the panes, leaving a one-cell gap between halves
    /// for the separator.
    fn place(&self, area: Rect, out: &mut Vec<(SessionId, Rect)>, gaps: &mut Vec<(Rect, Dir)>) {
        match self {
            Node::Pane(id) => out.push((*id, area)),
            Node::Split { dir, a, b } => {
                let (ra, gap, rb) = halves(area, *dir);
                a.place(ra, out, gaps);
                if gap.width > 0 && gap.height > 0 {
                    gaps.push((gap, *dir));
                }
                b.place(rb, out, gaps);
            }
        }
    }
}

fn halves(area: Rect, dir: Dir) -> (Rect, Rect, Rect) {
    match dir {
        Dir::Right => {
            let first = area.width.saturating_sub(1) / 2;
            let gap = area.width.min(1);
            let a = Rect {
                width: first,
                ..area
            };
            let g = Rect {
                x: area.x + first,
                width: gap,
                ..area
            };
            let b = Rect {
                x: area.x + first + gap,
                width: area.width - first - gap,
                ..area
            };
            (a, g, b)
        }
        Dir::Down => {
            let first = area.height.saturating_sub(1) / 2;
            let gap = area.height.min(1);
            let a = Rect {
                height: first,
                ..area
            };
            let g = Rect {
                y: area.y + first,
                height: gap,
                ..area
            };
            let b = Rect {
                y: area.y + first + gap,
                height: area.height - first - gap,
                ..area
            };
            (a, g, b)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    pub root: Node,
    /// The pane keys go to.
    pub focus: SessionId,
    /// The focused pane fills the tab.
    pub zoomed: bool,
    /// Shown after the tab's number; set from its right-click menu.
    pub name: Option<String>,
}

impl Tab {
    pub fn new(id: SessionId) -> Tab {
        Tab {
            root: Node::Pane(id),
            focus: id,
            zoomed: false,
            name: None,
        }
    }

    /// Shows `new` in the focused pane instead of what was there.
    pub fn replace_focused(&mut self, new: SessionId) {
        self.root.replace(self.focus, new);
        self.focus = new;
    }

    /// Splits the focused pane, putting `new` in the new half and focusing it.
    pub fn split_focused(&mut self, new: SessionId, dir: Dir) {
        self.root.split(self.focus, new, dir);
        self.focus = new;
        self.zoomed = false;
    }

    /// Removes a pane. Returns false if the tab is now empty.
    pub fn remove(&mut self, id: SessionId) -> bool {
        let root = std::mem::replace(&mut self.root, Node::Pane(id));
        match root.without(id) {
            Some(rest) => {
                self.root = rest;
                if self.focus == id {
                    self.focus = self.root.panes()[0];
                    self.zoomed = false;
                }
                true
            }
            None => false,
        }
    }

    /// Moves focus `delta` panes along the reading order, wrapping.
    pub fn cycle_focus(&mut self, delta: isize) {
        let panes = self.root.panes();
        let i = panes.iter().position(|&p| p == self.focus).unwrap_or(0) as isize;
        self.focus = panes[(i + delta).rem_euclid(panes.len() as isize) as usize];
    }

    /// Pane areas within `area`, plus separator cells. A lone (or zoomed)
    /// pane gets the whole area and no title row.
    pub fn areas(&self, area: Rect) -> (Vec<PaneArea>, Vec<(Rect, Dir)>) {
        let mut placed = Vec::new();
        let mut gaps = Vec::new();
        if self.zoomed {
            placed.push((self.focus, area));
        } else {
            self.root.place(area, &mut placed, &mut gaps);
        }
        let titled = placed.len() > 1;
        let panes = placed
            .into_iter()
            .map(|(session, r)| {
                let title_rows = u16::from(titled).min(r.height);
                PaneArea {
                    session,
                    title: Rect {
                        height: title_rows,
                        ..r
                    },
                    body: Rect {
                        y: r.y + title_rows,
                        height: r.height - title_rows,
                        ..r
                    },
                }
            })
            .collect();
        (panes, gaps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Rect = Rect {
        x: 26,
        y: 1,
        width: 81,
        height: 30,
    };

    #[test]
    fn splits_share_the_area_with_separators_and_titles() {
        let mut t = Tab::new(1);
        let (panes, gaps) = t.areas(AREA);
        assert_eq!(panes[0].body, AREA, "a lone pane has no title row");
        assert!(gaps.is_empty());

        t.split_focused(2, Dir::Right);
        t.split_focused(3, Dir::Down);
        assert_eq!(t.focus, 3);
        assert_eq!(t.root.panes(), vec![1, 2, 3]);
        let (panes, gaps) = t.areas(AREA);
        let by = |id| panes.iter().find(|p| p.session == id).unwrap();
        assert_eq!(by(1).body, Rect::new(26, 2, 40, 29));
        assert_eq!(by(1).title, Rect::new(26, 1, 40, 1));
        assert_eq!(by(2).title.x, 67);
        assert_eq!(by(2).body.width, 40);
        assert_eq!(by(3).title.y, 1 + 14 + 1);
        assert_eq!(gaps.len(), 2);
        assert_eq!(gaps[0], (Rect::new(66, 1, 1, 30), Dir::Right));
    }

    #[test]
    fn removing_panes_gives_space_to_siblings_and_moves_focus() {
        let mut t = Tab::new(1);
        t.split_focused(2, Dir::Right);
        t.split_focused(3, Dir::Down);
        assert!(t.remove(3));
        assert_eq!(t.focus, 1);
        assert_eq!(
            t.root,
            Node::Split {
                dir: Dir::Right,
                a: Box::new(Node::Pane(1)),
                b: Box::new(Node::Pane(2))
            }
        );
        assert!(t.remove(1));
        assert_eq!(t.root, Node::Pane(2));
        assert!(!t.remove(2), "tab is empty");
    }

    #[test]
    fn zoom_replace_and_cycle() {
        let mut t = Tab::new(1);
        t.split_focused(2, Dir::Down);
        t.zoomed = true;
        let (drawn, _) = t.areas(AREA);
        assert_eq!(drawn.len(), 1);
        assert_eq!((drawn[0].session, drawn[0].body), (2, AREA));
        t.replace_focused(5);
        assert_eq!(t.root.panes(), vec![1, 5]);
        t.cycle_focus(1);
        assert_eq!(t.focus, 1);
        t.cycle_focus(-1);
        assert_eq!(t.focus, 5);
        t.split_focused(6, Dir::Right);
        assert!(!t.zoomed, "splitting unzooms");
    }
}
