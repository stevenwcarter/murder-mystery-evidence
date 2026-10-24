//! HTML for the guest and host pages.

use maud::{DOCTYPE, Markup, html};

use crate::evidence::{EVIDENCE, Item, Round, for_round, visible};

/// The party's name, used as the page title and the masthead.
const TITLE: &str = "Jazz, Gin, Money & Murder";

/// Google Fonts: Limelight for display, Cormorant Garamond for body text.
const FONTS: &str = "https://fonts.googleapis.com/css2?family=Cormorant+Garamond:ital,wght@0,500;0,600;1,500&family=Limelight&display=swap";

/// Shown while no round has revealed any evidence.
const WAITING: &str =
    "The investigation is underway. Evidence will appear here as the detective uncovers it.";

/// Wraps a page's `<body>` in the shared document head.
fn layout(title: &str, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="color-scheme" content="dark";
                meta name="theme-color" content="#0b0b0e";
                title { (title) }
                link rel="preconnect" href="https://fonts.googleapis.com";
                link rel="preconnect" href="https://fonts.gstatic.com" crossorigin;
                link rel="stylesheet" href=(FONTS);
                link rel="stylesheet" href="/assets/style.css";
            }
            (body)
        }
    }
}

/// The title block shared by both pages, with a page-specific status line.
fn masthead(status: &Markup) -> Markup {
    html! {
        header.masthead {
            h1 { (TITLE) }
            p.subtitle { "Evidence Locker" }
            p.status { (status) }
        }
    }
}

/// The guest page: revealed rounds newest first, a sealed card per locked
/// round, and the full-screen image viewer driven by `app.js`.
pub fn guest(current: Round) -> Markup {
    let status = html! { "Round " (current) " of " (Round::MAX) };
    let any_revealed = EVIDENCE.iter().any(|item| visible(item, current));
    let body = html! {
        body data-round=(current) {
            (masthead(&status))
            main {
                @if !any_revealed {
                    section.round.notice { p { (WAITING) } }
                }
                @for round in Round::all().rev() {
                    (round_section(round, current))
                }
            }
            (viewer())
            footer {
                span #live { "connecting" }
            }
            script src="/assets/app.js" defer {}
        }
    };
    layout(TITLE, body)
}

/// One round on the guest page: sealed if it is still ahead, otherwise its
/// revealed items (nothing at all for a round without evidence).
///
/// A sealed round renders only its number, so no locked title or slug ever
/// reaches the page.
fn round_section(round: Round, current: Round) -> Markup {
    if round > current {
        return html! {
            section.round.sealed { p { "Round " (round) " · sealed" } }
        };
    }
    let mut items = for_round(round)
        .filter(|item| visible(item, current))
        .peekable();
    if items.peek().is_none() {
        return html! {};
    }
    html! {
        section.round.latest[round == current] id={ "round-" (round) } {
            h2 { "Round " (round) }
            div.cards {
                @for item in items { (card(item)) }
            }
        }
    }
}

/// A framed thumbnail that opens the viewer, with its title and a download link.
fn card(item: &Item) -> Markup {
    let href = format!("/evidence/{}", item.slug);
    html! {
        figure.card {
            a.view href=(href) data-title=(item.title) {
                img src=(href) alt=(item.title) loading="lazy";
            }
            figcaption { (item.title) }
            a.button href=(href) download { "Download" }
        }
    }
}

/// The full-screen viewer; `app.js` fills in the image and links on open.
fn viewer() -> Markup {
    html! {
        dialog #viewer aria-labelledby="viewer-title" {
            div.stage {
                img #viewer-image alt="";
            }
            div.viewer-bar {
                p #viewer-title {}
                a.button #viewer-download href="/" download { "Download" }
                a.button #viewer-open href="/" target="_blank" rel="noopener" { "Open image" }
                button.button #viewer-close type="button" autofocus { "Close" }
            }
        }
    }
}

/// The host page: one full-width button per round, the current one marked.
pub fn admin(current: Round) -> Markup {
    let status = html! { "Current round: " (current) };
    let body = html! {
        body.admin {
            (masthead(&status))
            main.controls {
                @for round in Round::all() {
                    form method="post" action="/admin/round" {
                        input type="hidden" name="round" value=(round);
                        @let is_current = round == current;
                        button
                            class=[is_current.then_some("current")]
                            aria-current=[is_current.then_some("true")]
                            type="submit"
                        {
                            "Round " (round)
                            @if round == Round::MIN { " · Nothing revealed" }
                        }
                    }
                }
            }
        }
    };
    layout(&format!("Detective · {TITLE}"), body)
}
