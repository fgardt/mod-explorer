use leptos::{ev::MouseEvent, html::Div, prelude::*};
use leptos_router::hooks::{use_location, use_navigate};

#[derive(Clone, Copy)]
enum SelectedLine {
    Line(usize),
    Block { start: usize, end: usize },
}

impl SelectedLine {
    fn from_hash(hash: &str) -> Option<Self> {
        let hash = hash.strip_prefix('#')?;
        if !hash.starts_with("L") {
            return None;
        }

        let parts = hash.split('-').collect::<Vec<_>>();
        match parts.len() {
            1 => parts
                .first()
                .unwrap()
                .strip_prefix('L')
                .unwrap_or_default()
                .parse()
                .ok()
                .map(Self::Line),
            2 => {
                let start = parts
                    .first()
                    .unwrap()
                    .strip_prefix('L')
                    .unwrap_or_default()
                    .parse()
                    .ok()?;
                let end = parts
                    .last()
                    .unwrap()
                    .strip_prefix('L')
                    .unwrap_or_default()
                    .parse()
                    .ok()?;

                if start > end {
                    return None;
                }

                Some(Self::Block { start, end })
            }
            _ => None,
        }
    }

    fn to_hash(self) -> String {
        match self {
            Self::Line(line) => format!("#L{line}"),
            Self::Block { start, end } => format!("#L{start}-L{end}"),
        }
    }

    fn handle_click(&mut self, line: usize, shift: bool) {
        match self {
            Self::Line(current) => {
                if shift {
                    let start = (*current).min(line);
                    let end = (*current).max(line);
                    *self = Self::Block { start, end };
                } else {
                    *current = line;
                }
            }
            Self::Block { start, end } => {
                if shift {
                    if line < *start {
                        *start = line;
                    } else if line > *end {
                        *end = line;
                    } else {
                        let dist_to_start = line - *start;
                        let dist_to_end = *end - line;

                        if dist_to_start <= dist_to_end {
                            *start = line;
                        } else {
                            *end = line;
                        }
                    }
                } else {
                    *self = Self::Line(line);
                }
            }
        }
    }

    const fn start_line(self) -> usize {
        match self {
            Self::Line(start) | Self::Block { start, .. } => start,
        }
    }

    const fn block_height(self) -> usize {
        match self {
            Self::Line(_) => 1,
            Self::Block { start, end } => end - start + 1,
        }
    }
}

fn click_handler(line: usize) -> impl Fn(MouseEvent) {
    move |ev| {
        let mut res = SelectedLine::Line(line);
        if let Some(mut selected) = SelectedLine::from_hash(&use_location().hash.read()) {
            selected.handle_click(line, ev.shift_key());
            res = selected;
        }

        let nav = use_navigate();
        nav(&res.to_hash(), Default::default());
    }
}

#[component]
pub fn LineNumbers(count: usize) -> impl IntoView {
    let numbers = (1..=count)
        .map(|i| {
            view! {
                <div on:click=click_handler(i)>{i}</div>
            }
        })
        .collect_view();

    view! {
        <LineHighlighter />
        <div class="line-numbers">
            {numbers}
        </div>
    }
}

#[component]
fn LineHighlighter() -> impl IntoView {
    let hash = use_location().hash;
    let selected = move || {
        let hash = hash.read();
        SelectedLine::from_hash(&hash)
    };

    let highlight_ref = NodeRef::<Div>::new();

    // Highlighter position & offset effect
    Effect::new(move |_| {
        let Some(h_div) = highlight_ref.get_untracked() else {
            return;
        };

        match selected() {
            None => {
                h_div.style("display:none");
            }
            Some(selected) => {
                let start = selected.start_line() - 1;
                let height = selected.block_height();

                let style = format!("margin-top:{start}lh;height:{height}lh");
                h_div.style(style);
            }
        }
    });

    Effect::new(move |_| {
        if untrack(selected).is_none() {
            return;
        }

        let Some(target) = highlight_ref.get_untracked() else {
            return;
        };

        target.scroll_into_view_with_bool(true);
    });

    view! {
        <div class="line-highlight" style="display:none" node_ref=highlight_ref />
    }
}
